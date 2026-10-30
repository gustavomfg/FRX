"""Explicit heuristic weak supervision, not gold and not Snorkel reproduction."""
import argparse
import hashlib
import json
import pathlib
import re
from proposals import propose


def vote(row):
    a,b=row["action_span"];text=row["text"]
    local=text[max(0,a-80):min(len(text),b+380)].lower()
    signals=[]
    if re.search(r"\b(try|could|should|please|might|may have)\b",local):signals.append("unknown")
    if re.search(r"\b(fixed|resolved|solved|works? (?:fine|now|perfectly)|crashes stopped)\b",local):signals.append("positive")
    if re.search(r"(didn.t (?:help|work|fix)|did not (?:help|work|fix)|still (?:crashes|freezes)|no (?:luck|change)|same problem)",local):signals.append("not-resolved")
    if re.search(r"(got worse|made .* worse|caused .* (?:crash|artifact)|introduced .* (?:crash|flicker))",local):signals.append("worsened")
    unique=set(signals)
    # Contradictions are not resolved by optimistic majority voting.
    return next(iter(unique)) if len(unique)==1 else None


def main(root,old):
    annotated=[]
    for name in ["train-events.jsonl","dev-events.jsonl","calibration-events.jsonl"]:
        annotated.extend(json.loads(x) for x in (root/name).read_text().splitlines())
    seen={r["url"] for r in annotated};train_apps={r["app_id"] for r in annotated if r["split"]=="train"}
    corpus=json.loads((old/"public/corpus.json").read_text());rows=[];counts={"conflicting_or_abstained":0}
    for bundle in corpus:
        issue=bundle["issue"];app=re.search(r"\((\d+)\)",issue["title"])
        if not app or app[1] not in train_apps:continue
        for c in bundle["comments"]:
            text=c.get("body") or ""
            if c["html_url"] in seen or len(text)>3200:continue
            for span in propose(text,limit=2):
                row=dict(text=text,action_span=span)
                y=vote(row)
                if y is None:counts["conflicting_or_abstained"]+=1;continue
                action=text[span[0]:span[1]].lower()
                d="unknown"
                for pattern,direction in [("disabl|turn.*off","disable"),("enabl|turn.*on","enable"),("uninstall|remov","remove"),("install","install"),("updat|upgrad|downgrad","update"),("replac|copied|rename","replace"),("switch","switch"),("setting|changed|set ","configure")]:
                    if re.search(pattern,action):d=direction;break
                if d=="unknown":continue
                row.update(id=hashlib.sha256((c["html_url"]+str(span)).encode()).hexdigest()[:16],
                    outcome=y,direction=d,modality="suggested" if y=="unknown" else "tested",
                    joint=False,result_span=None,url=c["html_url"],thread=issue["html_url"],
                    app_id=app[1],game=issue["title"],published_at=c.get("created_at"),
                    origin="weak-supervised-real",annotation="heuristic-uncertain-NOT-gold",split="train")
                rows.append(row)
    (root/"weak-events.jsonl").write_text(''.join(json.dumps(r)+'\n' for r in rows))
    counts["weak_rows"]=len(rows);(root/"weak-summary.json").write_text(json.dumps(counts,indent=2));print(counts)


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=pathlib.Path);p.add_argument("old",type=pathlib.Path)
    a=p.parse_args();main(a.root,a.old)
