"""Seeded, content-blind cohort draws. Reports counts; a test draw requires frozen code."""
import argparse
import json
import random
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "benchmark"))
import pilot  # noqa: E402

CACHE = Path.home() / ".cache/frx-ai-v8-20261030"
SEED = "frx-v8-20261030"
MAINTAINER = ("MEMBER", "OWNER", "COLLABORATOR")
LOG_LINE = re.compile(r"(?m)^.*(?:\berr:\w+:|\bwine: |Unhandled exception|undefined symbol|cannot open shared object|\bfixme:\w+:).*$")
PLAN = {"o8-dev": 30, "o8-test": 60, "t8-dev": 4, "t8-test": 12, "l8": 8}
TEST = {"o8-test", "t8-test", "l8"}


def excluded(root):
    known = pilot.known_threads()
    known |= {r["thread"] for r in pilot.read(pilot.ROOT / "sampling/strata.jsonl")}
    for name in PLAN:
        file = root / f"{name}-reserved.jsonl"
        if file.exists():
            known |= {r["thread"] for r in pilot.read(file)}
    return known


def corpus():
    titles = {r["number"]: r for r in pilot.read(pilot.CORPUS / "issues.jsonl")}
    by_thread = {}
    for c in pilot.read(pilot.CORPUS / "comments.jsonl"):
        t = pilot.thread_of(c)
        if t in titles:
            by_thread.setdefault(t, []).append(c)
    for rows in by_thread.values():
        rows.sort(key=lambda c: c["created_at"])
    return titles, by_thread


def comments(name, n, rng, known, titles, by_thread):
    pools = dict(outcome=[], several=[])
    for t, rows in by_thread.items():
        if t in known:
            continue
        for c in rows:
            if pilot.eligible(c) and pilot.VOCABULARY.search(c["body"]):
                pools["several" if pilot.stratum(c["body"]) == "several_actions" else "outcome"].append(c)
    chosen, used = [], set()
    third = n // 3
    for pool, size in ((pools["outcome"], n - third), (pools["several"], third)):
        got = 0
        for c in rng.sample(sorted(pool, key=lambda x: x["id"]), len(pool)):
            t = pilot.thread_of(c)
            if t in used:
                continue
            used.add(t)
            chosen.append(dict(id=f"{name}-{t}", thread=t, title=titles[t]["title"], url=c["html_url"], text=c["body"], author_association=c["author_association"], stratum="several" if pool is pools["several"] else "outcome"))
            got += 1
            if got == size:
                break
    return chosen


def threads(name, n, rng, known, titles, by_thread):
    """Threads with an issue report, a maintainer reply and the reporter answering afterwards."""
    pool = []
    for t, rows in by_thread.items():
        issue = titles[t]
        if t in known or not issue.get("body") or len(rows) < 3:
            continue
        owner = (issue.get("user") or {}).get("id")
        asked = next((i for i, c in enumerate(rows) if c["author_association"] in MAINTAINER), None)
        if asked is None or not any((c.get("user") or {}).get("id") == owner for c in rows[asked + 1:]):
            continue
        pool.append(t)
    picked = rng.sample(sorted(pool), n)
    out = []
    for t in picked:
        issue = titles[t]
        out.append(dict(id=f"{name}-{t}", thread=t, title=issue["title"], url=issue["html_url"], body=issue["body"], reporter=(issue.get("user") or {}).get("id"),
                        comments=[dict(id=c["id"], at=c["created_at"], role="maintainer" if c["author_association"] in MAINTAINER else "reporter" if (c.get("user") or {}).get("id") == (issue.get("user") or {}).get("id") else "other", url=c["html_url"], text=c["body"]) for c in by_thread[t]]))
    return out, len(pool)


def logs(name, n, rng, known, by_thread, titles):
    pool = []
    for t, rows in by_thread.items():
        if t in known:
            continue
        for c in rows:
            if len(LOG_LINE.findall(c["body"] or "")) >= 3 and len(c["body"]) < 12000:
                pool.append((t, c))
                break
    picked = rng.sample(sorted(pool, key=lambda x: x[1]["id"]), n)
    return [dict(id=f"{name}-{t}", thread=t, title=titles[t]["title"], url=c["html_url"], text=c["body"]) for t, c in picked], len(pool)


def draw(name, root=CACHE):
    root.mkdir(parents=True, exist_ok=True)
    dest = root / f"{name}-reserved.jsonl"
    if dest.exists():
        raise SystemExit(f"{name} already drawn")
    if name in TEST and not (root / "frozen.json").exists():
        raise SystemExit("a test cohort needs frozen code first")
    rng = random.Random(f"{SEED}/{name}")
    known = excluded(root)
    titles, by_thread = corpus()
    extra = {}
    if name.startswith("o8"):
        rows = comments(name, PLAN[name], rng, known, titles, by_thread)
    elif name.startswith("t8"):
        rows, extra["eligible_threads"] = threads(name, PLAN[name], rng, known, titles, by_thread)
    else:
        rows, extra["eligible_threads"] = logs(name, PLAN[name], rng, known, by_thread, titles)
    pilot.write_rows(dest, rows)
    print(json.dumps(dict(cohort=name, n=len(rows), sha256=pilot.sha(dest), excluded_threads=len(known), **extra)))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("cohort", choices=sorted(PLAN))
    draw(p.parse_args().cohort)
