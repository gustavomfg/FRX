"""Two search tools with one interface: the public GitHub API, and a frozen real snapshot.

Search hits are not readings: a hit carries title and metadata, and only `read` provides the
discussion. Both tools cut everything at a date, so a past case cannot see its future.
"""
import base64
import hashlib
import json
import math
import pickle
import re
import time
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import quote

import net

SNAPSHOT = Path.home() / ".cache/frx-ai-rnd-v7-20261030/corpus"
TOKEN = re.compile(r"[a-z0-9][a-z0-9_.+-]*[a-z0-9]|[a-z0-9]")
K1, B = 1.2, .75
DOCS = {"ValveSoftware/Proton": ["README.md"], "doitsujin/dxvk": ["README.md"], "HansKristian-Work/vkd3d-proton": ["README.md"], "Open-Wine-Components/umu-protonfixes": ["README.md"]}


def digest(text):
    return hashlib.sha256((text or "").encode()).hexdigest()


def tokens(text):
    return TOKEN.findall((text or "").lower())


def hit(repo, number, title, url, state, created, updated, association, author, score, body=None):
    return dict(repo=repo, number=number, title=title or "", url=url, state=state, created_at=created, updated_at=updated, author_association=association, author=author, score=score, snippet=(body or "")[:300])


class Frozen:
    """Real ValveSoftware/Proton data from the v7 snapshot. All terms must occur in the thread, ranked by BM25."""
    name = "frozen-snapshot"
    fetched_at = "2026-10-30T22:20:06Z"

    def __init__(self, root=SNAPSHOT, cache=net.CACHE):
        self.requests = 0
        cache = Path(cache)
        cache.mkdir(parents=True, exist_ok=True)
        file = cache / "frozen-index.pkl"
        if file.exists():
            self.issues, self.comments, self.post, self.lengths, self.avg = pickle.loads(file.read_bytes())
            return
        self.issues, self.comments = {}, defaultdict(list)
        for line in (Path(root) / "issues.jsonl").read_text().splitlines():
            i = json.loads(line)
            self.issues[i["number"]] = i
        for line in (Path(root) / "comments.jsonl").read_text().splitlines():
            c = json.loads(line)
            m = re.search(r"/issues/(\d+)", c["html_url"])
            if m and int(m.group(1)) in self.issues:
                self.comments[int(m.group(1))].append(c)
        for rows in self.comments.values():
            rows.sort(key=lambda c: c["created_at"])
        self.comments = dict(self.comments)
        self.post, self.lengths = defaultdict(dict), {}
        for n, i in self.issues.items():
            counts = Counter(tokens(i.get("title")) * 3 + tokens(i.get("body")))
            # Comments carry their dates: a term counts only from its first appearance.
            first = {t: i["created_at"] for t in counts}
            for c in self.comments.get(n, []):
                for t in tokens(c.get("body")):
                    counts[t] += 1
                    first.setdefault(t, c["created_at"])
            for t, tf in counts.items():
                self.post[t][n] = (tf, first[t])
            self.lengths[n] = sum(counts.values())
        self.post = dict(self.post)
        self.avg = sum(self.lengths.values()) / len(self.lengths)
        file.write_bytes(pickle.dumps((self.issues, self.comments, self.post, self.lengths, self.avg)))

    def search(self, terms, repo, until=None, exclude=(), limit=10):
        self.requests += 1
        until = until or "9999"
        if repo != "ValveSoftware/Proton":
            return dict(hits=[], total=0, repo=repo, note="the frozen snapshot only holds ValveSoftware/Proton")
        wanted = [t for term in terms for t in tokens(term)]
        if not wanted:
            return dict(hits=[], total=0, repo=repo)
        sets = []
        for t in wanted:
            sets.append({n for n, (_, first) in self.post.get(t, {}).items() if first < until})
        found = set.intersection(*sets) if sets else set()
        found = {n for n in found if self.issues[n]["created_at"] < until and n not in exclude}
        scores = {}
        total = len(self.issues)
        for t in wanted:
            docs = self.post.get(t, {})
            idf = math.log(1 + (total - len(docs) + .5) / (len(docs) + .5))
            for n in found:
                tf = docs[n][0]
                scores[n] = scores.get(n, 0) + idf * tf * (K1 + 1) / (tf + K1 * (1 - B + B * self.lengths[n] / self.avg))
        out = []
        for n in sorted(found, key=lambda x: (-scores[x], x))[:limit]:
            i = self.issues[n]
            out.append(hit(repo, n, i["title"], i["html_url"], i["state"], i["created_at"], i["updated_at"], i["author_association"], (i.get("user") or {}).get("login"), round(scores[n], 3), i.get("body")))
        return dict(hits=out, total=len(found), repo=repo)

    def read(self, repo, number, until=None):
        self.requests += 1
        until = until or "9999"
        i = self.issues.get(number)
        if repo != "ValveSoftware/Proton" or i is None or i["created_at"] >= until:
            raise net.NotFound("not in the frozen snapshot at this date")
        comments = [c for c in self.comments.get(number, []) if c["created_at"] < until]
        return thread(repo, i, comments, self.fetched_at, self.name)

    def doc(self, repo):
        raise net.NotFound("official documents are not in the frozen snapshot")


def thread(repo, issue, comments, fetched_at, source):
    """A discussion with provenance. The state is metadata: closed does not mean fixed."""
    parts = [dict(kind="issue", id=issue["number"], url=issue["html_url"], author=(issue.get("user") or {}).get("login"), association=issue.get("author_association"), created_at=issue["created_at"], updated_at=issue.get("updated_at"), text=issue.get("body") or "", sha256=digest(issue.get("body")))]
    for c in comments:
        parts.append(dict(kind="comment", id=c["id"], url=c["html_url"], author=(c.get("user") or {}).get("login"), association=c.get("author_association"), created_at=c["created_at"], updated_at=c.get("updated_at"), text=c.get("body") or "", sha256=digest(c.get("body"))))
    return dict(repo=repo, number=issue["number"], title=issue.get("title") or "", url=issue["html_url"], state=issue.get("state"), parts=parts, fetched_at=fetched_at, source=source)


class GitHub:
    """The public REST API, without authentication. Semantic search needs a token, so this is lexical."""
    name = "github-api"

    def __init__(self, client=None):
        self.client = client or net.Client()
        self.items = {}

    def search(self, terms, repo, until=None, exclude=(), limit=10):
        if repo not in net.REPOSITORIES:
            raise net.Blocked(f"repository not allowed: {repo}")
        text = " ".join(f'"{t}"' if " " in t else t for t in terms)
        qualifiers = f" repo:{repo} is:issue" + (f" created:<{until[:10]}" if until else "")
        url = f"https://api.github.com/search/issues?q={quote(text + qualifiers)}&per_page={limit}"
        data, fetched = self.client.get_json(url)
        out = []
        for i in data.get("items", []):
            if i.get("number") in exclude:
                continue
            self.items[(repo, i["number"])] = i
            out.append(hit(repo, i["number"], i.get("title"), i.get("html_url"), i.get("state"), i.get("created_at"), i.get("updated_at"), i.get("author_association"), (i.get("user") or {}).get("login"), i.get("score"), i.get("body")))
        return dict(hits=out, total=data.get("total_count", len(out)), repo=repo, incomplete=bool(data.get("incomplete_results")), url=url, fetched_at=fetched)

    def read(self, repo, number, until=None):
        if repo not in net.REPOSITORIES:
            raise net.Blocked(f"repository not allowed: {repo}")
        # A search item already holds the issue, so reading it costs only its comments.
        known = self.items.get((repo, number))
        issue, fetched = (known, now()) if known else self.client.get_json(f"https://api.github.com/repos/{repo}/issues/{int(number)}")
        comments = []
        if issue.get("comments"):
            rows, fetched = self.client.get_json(f"https://api.github.com/repos/{repo}/issues/{int(number)}/comments?per_page=100")
            comments = rows
        if until:
            if (issue.get("created_at") or "") >= until:
                raise net.NotFound("created after the case date")
            comments = [c for c in comments if c.get("created_at", "") < until]
        return thread(repo, issue, comments, fetched, self.name)

    def doc(self, repo):
        data, fetched = self.client.get_json(f"https://api.github.com/repos/{repo}/readme")
        text = base64.b64decode(data.get("content") or "").decode("utf-8", "replace") if data.get("encoding") == "base64" else ""
        return dict(repo=repo, number=0, title=f"{repo} README", url=data.get("html_url"), state="official-document", fetched_at=fetched, source=self.name,
                    parts=[dict(kind="document", id=data.get("sha"), url=data.get("html_url"), author=repo.split("/")[0], association="OFFICIAL_DOC", created_at=None, updated_at=None, text=text, sha256=digest(text))])


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
