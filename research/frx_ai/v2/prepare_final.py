"""Reserve a deterministic new-game test without exposing texts or gold labels.

One proposed action per comment; sample is conditional on this parser and is
not an estimate of all GitHub comments. Never used for training/selection.
"""
import argparse
import hashlib
import json
import pathlib
from proposals import propose


def main(root):
    dest=root/"final-candidates.jsonl"
    if dest.exists():raise ValueError("preserve first final reservation")
    corpus=json.loads((root/"reserved-final-sources.json").read_text())
    rows=[];seen=set();counts={}
    for bundle in corpus:
        candidates=[]
        for comment in bundle["comments"]:
            text=comment.get("body") or ""
            digest=hashlib.sha256(" ".join(text.split()).encode()).hexdigest()
            if len(text)>3200 or digest in seen:continue
            spans=propose(text)
            if not spans:continue
            seen.add(digest)
            # Stable sample of one action, independent of its unknown outcome.
            spans.sort(key=lambda x:hashlib.sha256((comment["html_url"]+str(x)).encode()).hexdigest())
            candidates.append(dict(id=digest[:16],url=comment["html_url"],
                thread=bundle["issue"]["html_url"],game=bundle["game"],app_id=bundle["app_id"],
                text=text,action_span=spans[0],published_at=comment.get("created_at"),
                origin="real-public-github",split="final",text_sha256=hashlib.sha256(text.encode()).hexdigest()))
        candidates.sort(key=lambda x:hashlib.sha256(("frx-v2-final"+x["url"]).encode()).hexdigest())
        picked=candidates[:12];rows.extend(picked);counts[bundle["app_id"]]=len(picked)
    dest.write_text(''.join(json.dumps(x)+'\n' for x in rows))
    manifest=dict(n=len(rows),per_game=counts,sha256=hashlib.sha256(dest.read_bytes()).hexdigest(),
                  selection="one deterministic parser proposal/comment, <=12 comments/game; no label conditioning")
    (root/"final-reservation.json").write_text(json.dumps(manifest,indent=2))
    print(json.dumps(manifest))


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=pathlib.Path);main(p.parse_args().root)
