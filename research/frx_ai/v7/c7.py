"""C7: complete investigation cases from reserved threads. Bodies are not used elsewhere.

`--prepare` writes inputs; `--run` compares policies once, after labels are locked.
The maintainer's own first request is recorded separately from agent judgement.
"""
import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "v6")]
from demos import MAINTAINER, label, reported  # noqa: E402


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def prepare(root):
    if (root / "c7-cases.jsonl").exists():
        raise SystemExit("preserve first C7 preparation")
    meta = json.loads((root / "reservation.json").read_text())
    wanted = {t["number"]: t["app_id"] for t in meta["c7_threads"]}
    issues = {}
    for line in (root / "corpus/issues.jsonl").read_text().splitlines():
        i = json.loads(line)
        if i["number"] in wanted:
            issues[i["number"]] = i
    replies = {}
    for line in (root / "corpus/comments.jsonl").read_text().splitlines():
        c = json.loads(line)
        n = int(c["issue_url"].rsplit("/", 1)[1])
        if n in wanted and c["author_association"] in MAINTAINER and "Issue transferred from" not in (c.get("body") or "")[:400]:
            replies.setdefault(n, []).append(c)
    rows = []
    for n, app in wanted.items():
        i = issues[n]
        first = min(replies.get(n, []), key=lambda c: c["created_at"], default=None)
        rows.append(dict(id=f"c7-{n}", thread=n, app_id=app, url=i["html_url"], game=re.sub(r"\s*\(\d+\)\s*$", "", i["title"] or ""), problem=reported(i.get("body"))[:6000], maintainer_labels=label(first.get("body")) if first else [], maintainer_url=first["html_url"] if first else None, maintainer_text=(first.get("body") or "")[:500] if first else None))
    (root / "c7-cases.jsonl").write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))
    print(len(rows), Counter(bool(r["maintainer_labels"]) for r in rows))


def score(case, gold, result):
    """Criteria written before execution; each is reported separately."""
    from knowledge import FORBIDDEN_WORDS
    got = set(result["understanding"]["symptoms"])
    top = result["next_check"]["id"] if result["next_check"] else None
    three = [c["id"] for c in result["next_checks"][:3]]
    hyps = [h["id"] for h in result["hypotheses"] if h["id"] != "unknown-cause"][:3]
    text = (case["problem"] or "")
    quotes_ok = all(s["quote"] in text or s["quote"] in (case.get("log") or "") for s in result["signals"] + result["symptoms"])
    recommendations = " ".join(c["question"] + " " + c["how"] for c in result["next_checks"]).lower()
    return dict(
        understanding=bool(got & set(gold["symptoms"])) if gold["symptoms"] else not got,
        evidence_literal=quotes_ok,
        hypotheses_pertinent=bool(set(hyps) & set(gold["hypotheses"])) if gold["hypotheses"] else not hyps,
        hypotheses_unconfirmed=result["confirmed_causes"] == [] and all(not h["confirmed"] for h in result["hypotheses"]),
        next_top1_agent=top in gold["next"],
        next_top3_agent=bool(set(three) & set(gold["next"])),
        safety=not any(w in recommendations for w in FORBIDDEN_WORDS) and not result["execution_authorized"] and not result["commands_executed"],
        efficiency=top not in gold.get("already_present", []),
        changes_state_first=bool(result["next_check"] and result["next_check"]["changes_state"]),
    )


def demo_class(check_id):
    from knowledge import CHECKS
    return CHECKS.get(check_id, {}).get("demo")


def run(root, old, replay=False):
    dest = root / ("c7-known-replay.json" if replay else "c7-results.json")
    if dest.exists():
        raise SystemExit("preserve existing C7 output")
    lock = json.loads((root / "c7-label-lock.json").read_text())
    if sha(root / "c7-gold.json") != lock["sha256"]:
        raise SystemExit("C7 labels changed after lock")
    import investigate as I
    from diagnostic import inspect
    cases = [json.loads(x) for x in (root / "c7-cases.jsonl").read_text().splitlines()]
    gold = json.loads((root / "c7-gold.json").read_text())
    prior = I.CasePrior(root)
    policies = {name: {} for name in ["always-log", "v6-rules", "v7-rules", "v7-case-prior", "v7-hybrid"]}
    records = []
    v6_map = {"collect-relevant-log": "collect-proton-log", "verify-component-symbol-versions": "resolve-library-path", "identify-proton": "collect-system-info", "identify-active-environment": "collect-system-info"}
    for case in cases:
        g = gold[case["id"]]
        base = dict(game=case["game"], problem=case["problem"], context=dict(app_id=case["app_id"]))
        rec = dict(id=case["id"], url=case["url"], maintainer_labels=case["maintainer_labels"], policies={})
        outs = {"v7-rules": I.analyse(base, "rules"), "v7-case-prior": I.analyse(base, "case-prior", prior), "v7-hybrid": I.analyse(base, "hybrid", prior)}
        for name, r in outs.items():
            s = score(case, g, r)
            s["next_top1_maintainer"] = demo_class(r["next_check"]["id"]) in case["maintainer_labels"] if r["next_check"] and case["maintainer_labels"] else None
            s["top"] = r["next_check"]["id"] if r["next_check"] else None
            rec["policies"][name] = s
        top = "collect-proton-log"
        rec["policies"]["always-log"] = dict(next_top1_agent=top in g["next"], next_top3_agent=top in g["next"], next_top1_maintainer=top in case["maintainer_labels"] if case["maintainer_labels"] else None, efficiency=top not in g.get("already_present", []), top=top)
        v6 = inspect(dict(context=dict(app_id=case["app_id"]), symptom=case["problem"][:1000], log=case["problem"], evidence=[], candidate_actions=[]))
        got = v6["next_checks"][0]["id"] if v6["next_checks"] else None
        top = v6_map.get(got, got)
        rec["policies"]["v6-rules"] = dict(next_top1_agent=top in g["next"], next_top3_agent=bool({v6_map.get(c["id"], c["id"]) for c in v6["next_checks"][:3]} & set(g["next"])), next_top1_maintainer=demo_class(top) in case["maintainer_labels"] if case["maintainer_labels"] and top else None, efficiency=top not in g.get("already_present", []), top=got)
        records.append(rec)
    keys = ["understanding", "evidence_literal", "hypotheses_pertinent", "hypotheses_unconfirmed", "next_top1_agent", "next_top3_agent", "next_top1_maintainer", "safety", "efficiency", "changes_state_first"]
    summary = {}
    for name in policies:
        row = {}
        for k in keys:
            values = [r["policies"][name].get(k) for r in records if r["policies"][name].get(k) is not None]
            if values:
                row[k] = [sum(bool(v) for v in values), len(values)]
        row["top_counter"] = Counter(r["policies"][name]["top"] for r in records).most_common(6)
        summary[name] = row
    with dest.open("x") as f:
        json.dump(dict(status="KNOWN REPLAY after post-test fixes; not an independent evaluation" if replay else "first C7 evaluation; agent milestones plus maintainer's first request", summary=summary, records=records, label_lock=lock, limitation="Agent-written milestones; one first-post view without attached logs."), f, indent=1, default=str)
    print(json.dumps(summary, indent=0, default=str))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--old", type=Path)
    p.add_argument("--prepare", action="store_true")
    p.add_argument("--lock-labels", action="store_true")
    p.add_argument("--replay", action="store_true")
    a = p.parse_args()
    if a.prepare:
        prepare(a.root)
    elif a.lock_labels:
        (a.root / "c7-label-lock.json").write_text(json.dumps(dict(sha256=sha(a.root / "c7-gold.json"), predictions_seen=False, provisional_agent_labels=True), indent=1))
    else:
        run(a.root, a.old, a.replay)
