"""Reserve P7 (language test) and C7 (case test) before the generator is frozen.

Prints counts and identifiers only; bodies stay on disk unread until labels start.
"""
import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
SEED = "frx-v7-20261030"
ACTION = re.compile(r"(?i)\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|set(?:ting)?|tried|try|trying|launch\w*|using|used|ran|restart\w*|reboot\w*|test\w*|add\w*|delet\w*|turn\w*)\b")
RESULT = re.compile(r"(?i)\b(?:work\w*|fix\w*|resolv\w*|crash\w*|hang\w*|fail\w*|worse|better|stopp\w*|same|help\w*|success\w*|freez\w*|stutter\w*|error\w*|launch\w*|run\w*)\b")
REQUEST = re.compile(r"(?i)\b(?:please|could you|have you|can you|should)\b")
TITLE = re.compile(r"\((\d{3,10})\)\s*$")


def key(text):
    return hashlib.sha256((SEED + ":" + text).encode()).hexdigest()


def used_app_ids():
    """Every AppID appearing in a prior split, snapshot or known cache."""
    found = set()
    roots = list((HERE.parent).glob("**/snapshot/*.json")) + list((HERE.parent).glob("v*/*.py"))
    for path in roots:
        try:
            found.update(re.findall(r'"(?:source_thread_)?app_id": ?"(\d{3,10})"', path.read_text(errors="ignore")))
            found.update(re.findall(r"\('(\d{3,10})',\s*\d+\)", path.read_text(errors="ignore")))
        except OSError:
            continue
    for cache in Path.home().glob(".cache/frx-ai-*"):
        for name in cache.glob("*gold*.jsonl"):
            found.update(re.findall(r'"app_id": ?"(\d{3,10})"', name.read_text(errors="ignore")))
    return found


def paragraphs(text):
    for m in re.finditer(r"[^\n].*?(?=\r?\n\s*\r?\n|\Z)", text, re.S):
        a, b = m.span()
        while a < b and text[a].isspace():
            a += 1
        while b > a and text[b - 1].isspace():
            b -= 1
        part = text[a:b]
        if 40 <= len(part) <= 1300 and not part.startswith(("<!--", "```", "![", "<img", "# System", "## System", "# Compatibility")):
            yield a, b


def units_for(number, app, title, comments, seen):
    pool = []
    for c in comments:
        text = c.get("body") or ""
        if len(text) > 200000 or c["user"]["type"] == "Bot":
            continue
        options = list(paragraphs(text))
        if not options:
            continue
        a, b = max(options, key=lambda p: (len(ACTION.findall(text[slice(*p)])) >= 1 and bool(RESULT.search(text[slice(*p)])), len(ACTION.findall(text[slice(*p)]))))
        piece = text[a:b]
        digest = hashlib.sha256(piece.encode()).hexdigest()
        if digest in seen:
            continue
        actions = len(ACTION.findall(piece))
        pool.append(dict(comment=c, a=a, b=b, text=piece, digest=digest, actions=actions, rich=actions >= 1 and bool(RESULT.search(piece)), request=bool(REQUEST.search(piece)), key=key(str(c["id"]))))
    pool.sort(key=lambda x: x["key"])
    chosen, joint = [], []
    def take(rule, n, into):
        for item in pool:
            if len([x for x in into if x["stratum"] == rule[0]]) >= n:
                return
            if item["comment"]["id"] in {x["comment"]["id"] for x in chosen + joint}:
                continue
            if rule[1](item):
                into.append(dict(item, stratum=rule[0]))
    take(("multi-action", lambda x: x["actions"] >= 3 and x["rich"]), 3, joint)
    take(("rich", lambda x: x["rich"]), 6, chosen)
    take(("multi", lambda x: x["actions"] >= 2), 2, chosen)
    take(("request", lambda x: x["request"]), 1, chosen)
    take(("residual", lambda x: True), 1, chosen)
    def row(item, prefix, i):
        c = item["comment"]
        return dict(id=f"{prefix}-{number}-{i}", app_id=app, source_thread_app_id=app, thread=number, title=title, url=c["html_url"], document_id=str(c["id"]), reporter_id="github:" + str(c["user"]["id"]), author_association=c["author_association"], source_sha256=hashlib.sha256((c.get("body") or "").encode()).hexdigest(), source_offset=item["a"], paragraph_sha256=item["digest"], published_at=c["created_at"], stratum=item["stratum"], text=item["text"], source_text=c["body"])
    return [row(x, "p7m", i) for i, x in enumerate(chosen)], [row(x, "p7j", i) for i, x in enumerate(joint)]


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    a = p.parse_args()
    if (a.root / "reservation.json").exists():
        raise SystemExit("preserve first reservation")
    issues = [json.loads(x) for x in (a.root / "corpus/issues.jsonl").read_text().splitlines()]
    comments = defaultdict(list)
    for x in (a.root / "corpus/comments.jsonl").read_text().splitlines():
        c = json.loads(x)
        comments[int(c["issue_url"].rsplit("/", 1)[1])].append(c)
    used = used_app_ids()
    games = []
    for i in issues:
        m = TITLE.search(i["title"] or "")
        if m and not i.get("pull_request") and m[1] not in used:
            games.append((i, m[1]))
    games = sorted(games, key=lambda g: key(str(g[0]["number"])))
    apps = {}
    taken = set()
    p7 = []
    cases = []
    for i, app in games:
        if app in taken:
            continue
        n = len([c for c in comments[i["number"]] if c["user"]["type"] != "Bot"])
        if len(p7) < 8 and n >= 8:
            p7.append((i, app))
            taken.add(app)
        elif len(cases) < 24 and n >= 2 and i.get("body") and any(c["author_association"] in ("MEMBER", "COLLABORATOR") for c in comments[i["number"]]):
            cases.append((i, app))
            taken.add(app)
        if len(p7) == 8 and len(cases) == 24:
            break
    seen = set()
    main, joint = [], []
    for i, app in p7:
        m, j = units_for(i["number"], app, i["title"], sorted(comments[i["number"]], key=lambda c: c["created_at"]), seen)
        main += m
        joint += j
    with (a.root / "p7-reserved.jsonl").open("x") as f:
        for r in main + joint:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    meta = dict(
        seed=SEED, used_app_ids=len(used), eligible_threads=len(games),
        p7_threads=[dict(number=i["number"], app_id=app) for i, app in p7],
        c7_threads=[dict(number=i["number"], app_id=app) for i, app in cases],
        p7_main_units=len(main), p7_joint_units=len(joint),
        reserved_app_ids=sorted(taken),
        p7_sha256=hashlib.sha256((a.root / "p7-reserved.jsonl").read_bytes()).hexdigest(),
    )
    (a.root / "reservation.json").write_text(json.dumps(meta, indent=1))
    print(json.dumps({k: v for k, v in meta.items() if k != "reserved_app_ids"}, indent=1))
