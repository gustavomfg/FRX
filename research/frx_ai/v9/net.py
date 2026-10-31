"""Read-only network tool. Every limit is enforced here, outside any decision about what to search.

GET only, HTTPS only, allowed hosts and routes, no private destinations, validated redirects,
bounded size and time, a request budget, a disk cache and honest handling of rate limits.
No credentials are read or sent.
"""
import hashlib
import http.client
import ipaddress
import json
import re
import socket
import ssl
import time
from pathlib import Path
from urllib.parse import urlsplit

CACHE = Path.home() / ".cache/frx-ai-v9-20261031"
ALLOWED_HOSTS = {"api.github.com"}
REPOSITORIES = {"ValveSoftware/Proton", "doitsujin/dxvk", "HansKristian-Work/vkd3d-proton", "Open-Wine-Components/umu-protonfixes", "Open-Wine-Components/umu-database"}
ROUTES = [re.compile(r"^/search/issues$"), re.compile(r"^/repos/[\w.-]+/[\w.-]+/issues/\d+(?:/comments)?$"), re.compile(r"^/repos/[\w.-]+/[\w.-]+/readme$"), re.compile(r"^/rate_limit$")]
MAX_BYTES = 2_000_000
MAX_REDIRECTS = 2
TIMEOUT = 15
MIN_SEARCH_INTERVAL = 6.5
USER_AGENT = "frx-ai-v9-research-lab"


class Blocked(Exception):
    """The tool refused a request before or after contacting the network."""


class RateLimited(Exception):
    def __init__(self, reset, message="rate limited"):
        super().__init__(message)
        self.reset = reset


class Unavailable(Exception):
    """Network failure, timeout, or an unusable answer."""


class NotFound(Exception):
    pass


def resolve_public(host, resolver=socket.getaddrinfo):
    """Every resolved address must be public; a private answer blocks the request."""
    try:
        infos = resolver(host, 443, type=socket.SOCK_STREAM)
    except socket.gaierror as e:
        raise Unavailable(f"dns: {e}") from e
    for info in infos:
        ip = ipaddress.ip_address(info[4][0])
        if not ip.is_global:
            raise Blocked(f"private destination {ip}")
    return [i[4][0] for i in infos]


def validate_url(url):
    parts = urlsplit(url)
    if parts.scheme != "https":
        raise Blocked("only https")
    if parts.username or parts.password:
        raise Blocked("credentials in url")
    if parts.port not in (None, 443):
        raise Blocked("port not allowed")
    host = (parts.hostname or "").lower()
    if host not in ALLOWED_HOSTS:
        raise Blocked(f"host not allowed: {host}")
    if not any(r.match(parts.path) for r in ROUTES):
        raise Blocked(f"route not allowed: {parts.path}")
    m = re.match(r"^/repos/([\w.-]+/[\w.-]+)/", parts.path)
    if m and m.group(1) not in REPOSITORIES:
        raise Blocked(f"repository not allowed: {m.group(1)}")
    if ".." in parts.path:
        raise Blocked("path traversal")
    return parts


class Response:
    def __init__(self, status, headers, body):
        self.status, self.headers, self.body = status, {k.lower(): v for k, v in headers}, body


def https_get(parts, headers, timeout=TIMEOUT):
    """One GET with no automatic redirects, reading at most MAX_BYTES + 1."""
    connection = http.client.HTTPSConnection(parts.hostname, 443, timeout=timeout, context=ssl.create_default_context())
    try:
        target = parts.path + (f"?{parts.query}" if parts.query else "")
        connection.request("GET", target, headers=headers)
        reply = connection.getresponse()
        body = reply.read(MAX_BYTES + 1)
        return Response(reply.status, reply.getheaders(), body)
    finally:
        connection.close()


class Client:
    """Budgeted, cached and logged. `transport` and `resolver` exist so failures can be tested offline."""

    def __init__(self, max_requests=30, cache=CACHE / "net", transport=https_get, resolver=socket.getaddrinfo, sleep=time.sleep, offline=False):
        self.max_requests, self.transport, self.resolver, self.sleep, self.offline = max_requests, transport, resolver, sleep, offline
        self.cache = Path(cache)
        self.calls, self.rejections, self.network_requests, self.last_search = [], [], 0, 0.0

    def _log(self, url, outcome, **extra):
        self.calls.append(dict(url=url, outcome=outcome, at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **extra))

    def _reject(self, url, reason):
        self.rejections.append(dict(url=str(url)[:200], reason=str(reason)[:200]))

    def get_json(self, url):
        try:
            validate_url(url)
        except Blocked as e:
            self._reject(url, e)
            raise
        key = hashlib.sha256(url.encode()).hexdigest()
        file = self.cache / f"{key}.json"
        if file.exists():
            stored = json.loads(file.read_text())
            self._log(url, "cache", fetched_at=stored["fetched_at"])
            return stored["data"], stored["fetched_at"]
        if self.offline:
            raise Unavailable("offline mode and not in cache")
        data = self._fetch(url)
        fetched = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        self.cache.mkdir(parents=True, exist_ok=True)
        file.write_text(json.dumps(dict(url=url, fetched_at=fetched, data=data)))
        return data, fetched

    def _fetch(self, url):
        current = url
        for hop in range(MAX_REDIRECTS + 1):
            parts = validate_url(current)
            if self.network_requests >= self.max_requests:
                self._reject(current, "request budget exhausted")
                raise Blocked("request budget exhausted")
            try:
                resolve_public(parts.hostname, self.resolver)
            except Blocked as e:
                self._reject(current, e)
                raise
            if parts.path == "/search/issues":
                self._pace_search()
            self.network_requests += 1
            try:
                reply = self.transport(parts, {"Accept": "application/vnd.github+json", "User-Agent": USER_AGENT, "X-GitHub-Api-Version": "2022-11-28"})
            except (OSError, http.client.HTTPException) as e:
                self._log(current, "network-failure", detail=str(e)[:120])
                raise Unavailable(str(e)) from e
            self._log(current, str(reply.status), remaining=reply.headers.get("x-ratelimit-remaining"))
            if reply.status in (301, 302, 307, 308):
                target = reply.headers.get("location", "")
                try:
                    validate_url(target)
                except Blocked as e:
                    self._reject(target, f"redirect refused: {e}")
                    raise
                current = target
                continue
            return self._decode(reply)
        self._reject(url, "too many redirects")
        raise Blocked("too many redirects")

    def _pace_search(self):
        """Searches stay spaced across processes too: the /rate_limit numbers do not show secondary limits."""
        stamp = self.cache / "last-search"
        try:
            last = float(stamp.read_text())
        except (OSError, ValueError):
            last = 0.0
        wait = MIN_SEARCH_INTERVAL - (time.time() - last)
        if 0 < wait <= MIN_SEARCH_INTERVAL:
            self.sleep(wait)
        self.cache.mkdir(parents=True, exist_ok=True)
        stamp.write_text(str(time.time()))

    def _decode(self, reply):
        if reply.status in (403, 429) and (reply.headers.get("x-ratelimit-remaining") == "0" or "retry-after" in reply.headers):
            retry = reply.headers.get("retry-after")
            reset = time.time() + int(retry) if retry and retry.isdigit() else float(reply.headers.get("x-ratelimit-reset") or time.time() + 60)
            raise RateLimited(reset)
        if reply.status == 404:
            raise NotFound("404")
        if reply.status != 200:
            raise Unavailable(f"http {reply.status}")
        if len(reply.body) > MAX_BYTES:
            self._reject("response", "response too large")
            raise Blocked("response too large")
        if "json" not in reply.headers.get("content-type", "application/json"):
            raise Unavailable("unexpected content type")
        try:
            return json.loads(reply.body)
        except ValueError as e:
            raise Unavailable("incomplete or invalid response") from e
