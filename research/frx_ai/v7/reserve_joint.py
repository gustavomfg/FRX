"""J7: challenge units with several first-person changes and a result nearby.

Selected mechanically after the generator froze. Enriched on purpose: it tests
procedure structure and says nothing about how common such reports are.
"""
import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

from reserve import TITLE, key, paragraphs, used_app_ids

OPS = re.compile(r"(?i)\b(?:installed|uninstalled|reinstalled|removed|deleted|disabled|enabled|changed|switched|updated|upgraded|downgraded|replaced|renamed|copied|added|set|ran|tried|turned|cleared|rebooted|restarted|verified|forced)\b")
RESULT = re.compile(r"(?i)\b(?:works?|worked|working|fixed|solved|launch(?:es|ed)|runs?|started|boots?|crash\w*|still|same|nothing|no luck|didn't|doesn't|won't)\b")
FIRST = re.compile(r"(?i)\b(?:i|i've|we|my)\b")


def main(root, size=40, per_thread=2):
    if (root / "j7-reserved.jsonl").exists():
        raise SystemExit("preserve first reservation")
    meta = json.loads((root / "reservation.json").read_text())
    blocked = used_app_ids() | set(meta["reserved_app_ids"])
    issues = {}
    for line in (root / "corpus/issues.jsonl").read_text().splitlines():
        i = json.loads(line)
        m = TITLE.search(i["title"] or "")
        if m and not i.get("pull_request") and m[1] not in blocked:
            issues[i["number"]] = (i, m[1])
    pool = []
    for line in (root / "corpus/comments.jsonl").read_text().splitlines():
        c = json.loads(line)
        number = int(c["issue_url"].rsplit("/", 1)[1])
        if number not in issues or c["author_association"] in ("MEMBER", "COLLABORATOR", "OWNER") or c["user"]["type"] == "Bot":
            continue
        text = c.get("body") or ""
        for a, b in paragraphs(text):
            piece = text[a:b]
            if piece.startswith(">") or not FIRST.search(piece):
                continue
            hits = OPS.findall(piece)
            last = list(OPS.finditer(piece))
            if len({h.lower() for h in hits}) >= 2 and last and RESULT.search(piece[last[1].end():]):
                pool.append((key("j7:" + str(c["id"]) + ":" + str(a)), c, number, a, b, piece))
    pool.sort(key=lambda x: x[0])
    seen, used, chosen = Counter(), set(), []
    for _, c, number, a, b, piece in pool:
        digest = hashlib.sha256(piece.encode()).hexdigest()
        app = issues[number][1]
        if seen[app] >= per_thread or digest in used:
            continue
        seen[app] += 1
        used.add(digest)
        chosen.append(dict(id=f"j7-{len(chosen)}", app_id=app, source_thread_app_id=app, thread=number, title=issues[number][0]["title"], url=c["html_url"], document_id=str(c["id"]), reporter_id="github:" + str(c["user"]["id"]), author_association=c["author_association"], source_sha256=hashlib.sha256((c.get("body") or "").encode()).hexdigest(), source_offset=a, paragraph_sha256=digest, published_at=c["created_at"], stratum="joint-challenge", text=piece))
        if len(chosen) == size:
            break
    with (root / "j7-reserved.jsonl").open("x") as f:
        for r in chosen:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    meta["j7_app_ids"] = sorted(seen)
    meta["j7_units"] = len(chosen)
    meta["j7_pool"] = len(pool)
    (root / "reservation.json").write_text(json.dumps(meta, indent=1))
    print(len(chosen), len(pool), len(seen))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    main(p.parse_args().root)
