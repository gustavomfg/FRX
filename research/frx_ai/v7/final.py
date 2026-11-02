"""One prospective evaluation on P7 and J7. Refuses to run twice or on changed code."""
import argparse
import copy
import hashlib
import json
import statistics
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import freeze  # noqa: E402
import procedures  # noqa: E402
from bridge import CompactSyntax, Syntax, prior_events, prior_reconstruct, read  # noqa: E402
from classifier import Classifier  # noqa: E402
from engine import Interpreter  # noqa: E402
from evaluate import evaluate, mapping, overlap  # noqa: E402
from l1_adapter import rescore  # noqa: E402
from waterfall import summarise  # noqa: E402
import events as _v5_events  # noqa: E402

# Amendment recorded in snapshot/amendments.json: the inherited v5 state rule can
# build an inverted span when a governor follows its verb. Order the two offsets;
# no threshold, rule or label changes, and no metric existed when this was added.
_inherited_evidence = _v5_events.evidence
_v5_events.evidence = lambda text, a, b: _inherited_evidence(text, min(a, b), max(a, b))


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def lock(root):
    dest = root / "label-lock.json"
    if dest.exists():
        raise SystemExit("preserve label lock")
    dest.write_text(json.dumps(dict(p7=sha(root / "p7-gold.jsonl"), j7=sha(root / "j7-gold.jsonl"), locked_at=datetime.now(timezone.utc).isoformat(), predictions_seen=False, provisional_agent_labels=True, human_review=False), indent=1))


def lenient(row, pred):
    """Declared variant: a light verb and its content verb name the same action."""
    out = copy.copy(pred)
    out["events"] = copy.deepcopy(pred["events"])
    for e in out["events"]:
        spans = [e["trigger"]["span"]] + ([e["light_trigger"]["span"]] if e.get("light_trigger") else [])
        for g in row["events"]:
            refs = [g["trigger"]] + ([g["trigger_alt"]] if g.get("trigger_alt") else [])
            if any(overlap(s, r) for s in spans for r in refs):
                e["trigger"] = dict(e["trigger"], span=list(g["trigger"]))
                break
    return out


def relations(row):
    return [o for o in row.get("observations", []) if o["event_indices"] and o["outcome"] != "unknown"]


def joint_metrics(rows, preds):
    n = dict(gold_groups=0, group_recovered=0, group_proposed=0, individual_credit_errors=0, individual_positive_credit_errors=0, structure_correct=0, structure_scored=0, partial_group_claims=0, gold_singles=0, single_recovered=0, causal_claims=0)
    for row, pred in zip(rows, preds):
        links = mapping(row, pred)
        for c in pred.get("observations", []):
            n["causal_claims"] += bool(c.get("causality_confirmed"))
        for g in relations(row):
            owners = sorted(g["event_indices"])
            big = len(owners) > 1
            n["gold_groups" if big else "gold_singles"] += 1
            claims = [c for c in pred.get("observations", []) if overlap(c["evidence"]["span"], g["span"]) and c["outcome"] == g["outcome"]]
            exact = [c for c in claims if sorted(links.get(i, -1) for i in c["event_indices"]) == owners]
            detected = pred.get("detected_observations", [])
            proposed = any(
                sorted(links.get(i, -1) for i in p["event_indices"]) == owners
                and p["observation"] < len(detected)
                and overlap(detected[p["observation"]]["evidence"]["span"], g["span"])
                and detected[p["observation"]]["outcome"] == g["outcome"]
                for p in pred.get("proposals", [])
            )
            if big:
                n["group_recovered"] += bool(exact)
                n["group_proposed"] += proposed
                for c in claims:
                    own = [links.get(i, -1) for i in c["event_indices"]]
                    if len(own) == 1 and own[0] in owners:
                        n["individual_credit_errors"] += 1
                        n["individual_positive_credit_errors"] += c["outcome"] == "improved"
                    elif 1 < len(own) and sorted(own) != owners and set(own) & set(owners):
                        n["partial_group_claims"] += 1
            else:
                n["single_recovered"] += bool(exact)
            if exact and "structure" in exact[0] and big:
                n["structure_scored"] += 1
                n["structure_correct"] += exact[0]["structure"] == g.get("structure")
    return n


def candidate_metrics(rows, preds):
    per_doc = [len(p.get("proposals", [])) for p in preds]
    total = sum(per_doc)
    useful = 0
    for row, pred in zip(rows, preds):
        links = mapping(row, pred)
        detected = pred.get("detected_observations", [])
        for p in pred.get("proposals", []):
            owners = sorted(links.get(i, -1) for i in p["event_indices"])
            if p["observation"] >= len(detected):
                continue
            o = detected[p["observation"]]
            useful += any(sorted(g["event_indices"]) == owners and g["outcome"] == o["outcome"] and overlap(o["evidence"]["span"], g["span"]) for g in relations(row))
    return dict(candidates=total, per_doc_mean=total / max(1, len(rows)), per_doc_median=statistics.median(per_doc) if per_doc else 0, per_doc_max=max(per_doc, default=0), candidate_precision=useful / total if total else None)


def clean(pred):
    return {k: v for k, v in pred.items() if k != "parsed"}


def report(rows, preds):
    out = {}
    for name, ps in preds.items():
        strict = evaluate(rows, ps)
        loose = evaluate(rows, [lenient(r, p) for r, p in zip(rows, ps)])
        keep = ["gold_relations", "proposable_relations", "relations", "relation_correct", "relation_precision", "relation_recall", "false_success", "negative_correct", "joint_claims", "joint_correct", "gold_performed", "performed", "performed_correct", "performed_precision", "performed_recall", "false_performed"]
        s, _ = summarise(rows, ps)
        out[name] = dict(strict={k: strict[k] for k in keep}, lenient={k: loose[k] for k in keep}, joint=joint_metrics(rows, ps), candidates=candidate_metrics(rows, ps), first_failure=s["first_failure"])
    return out


def bootstrap(rows, preds, a="v7", b="v6-selected", seed=707, rounds=4000):
    threads = sorted({r["thread"] for r in rows})
    per = {}
    for t in threads:
        ids = [i for i, r in enumerate(rows) if r["thread"] == t]
        sub = [rows[i] for i in ids]
        ma = evaluate(sub, [preds[a][i] for i in ids])
        mb = evaluate(sub, [preds[b][i] for i in ids])
        per[t] = (ma["gold_relations"], ma["proposable_relations"], mb["proposable_relations"], ma["relation_correct"], mb["relation_correct"])
    rng = np.random.default_rng(seed)
    cand, rec = [], []
    for _ in range(rounds):
        sample = rng.choice(threads, len(threads))
        g = sum(per[t][0] for t in sample) or 1
        cand.append(sum(per[t][1] - per[t][2] for t in sample) / g)
        rec.append(sum(per[t][3] - per[t][4] for t in sample) / g)
    return dict(threads=len(threads), candidate_recall_gain=np.quantile(cand, [0.025, 0.975]).tolist(), relation_recall_gain=np.quantile(rec, [0.025, 0.975]).tolist(), per_thread={str(k): v for k, v in per.items()})


def run(root):
    freeze_state = json.loads((root / "frozen.json").read_text())
    changed = [k for k, v in freeze.hashes().items() if freeze_state["files"].get(k) != v]
    if changed:
        raise SystemExit(f"frozen code changed: {changed}")
    dest = root / "final-results.json"
    if dest.exists():
        raise SystemExit("preserve first final evaluation")
    label = json.loads((root / "label-lock.json").read_text())
    if sha(root / "p7-gold.jsonl") != label["p7"] or sha(root / "j7-gold.jsonl") != label["j7"]:
        raise SystemExit("labels changed after lock")
    rows = read(root / "p7-gold.jsonl") + read(root / "j7-gold.jsonl")
    catalog = json.loads((root / "catalog.json").read_text())
    selection = json.loads((root / "selection.json").read_text())
    threshold = freeze_state["selection"]["threshold"]
    small, large = CompactSyntax(root), Syntax(root)
    l1 = Classifier(root / selection["model"])
    v6 = Interpreter(small, catalog, l1, selection["threshold"], mode="trained-structured-verifier")
    v6s = Interpreter(small, catalog, threshold=0.5)
    preds, timing = {}, {}
    start = time.perf_counter()
    preds["v6-selected"] = [v6.analyze(r) for r in rows]
    timing["v6-selected"] = (time.perf_counter() - start) / len(rows) * 1000
    preds["v6-structural"] = [v6s.analyze(r) for r in rows]
    start = time.perf_counter()
    preds["v7"] = [procedures.analyze(r, small, catalog, threshold) for r in rows]
    timing["v7"] = (time.perf_counter() - start) / len(rows) * 1000
    preds["v7-candidates+v6-L1"] = [rescore(r, p, l1, selection["threshold"]) for r, p in zip(rows, preds["v7"])]
    preds["v5-stanza"] = [prior_reconstruct(r["text"], p, prior_events(r["text"], p)) for r in rows for p in [large.parse(r["text"])]]
    groups = {"P7-M": [i for i, r in enumerate(rows) if r["id"].startswith("p7m")], "P7-J": [i for i, r in enumerate(rows) if r["id"].startswith("p7j")], "J7": [i for i, r in enumerate(rows) if r["id"].startswith("j7")]}
    results = {g: report([rows[i] for i in ids], {k: [v[i] for i in ids] for k, v in preds.items()}) for g, ids in groups.items()}
    boots = {g: bootstrap([rows[i] for i in ids], {k: [v[i] for i in ids] for k, v in preds.items()}) for g, ids in groups.items()}
    out = dict(status="first v7 evaluation; agent-provisional labels", results=results, bootstrap=boots, timing_ms_per_doc_cold=timing, label_lock=label, frozen=freeze_state["selection"], limitations=["No independent human review of labels.", "P7 and J7 are enriched and do not estimate prevalence.", "Labels and generator were produced by the same agent.", "Reported association is not causal evidence."])
    with dest.open("x") as f:
        json.dump(out, f, indent=1)
    (root / "final-predictions.json").write_text(json.dumps({k: [clean(x) for x in v] for k, v in preds.items()}, default=str))
    for g, r in results.items():
        print("==", g, "docs", len(groups[g]))
        for k, m in r.items():
            s, l, j, c = m["strict"], m["lenient"], m["joint"], m["candidates"]
            print(f"{k:22} cand {s['proposable_relations']}/{s['gold_relations']} (len {l['proposable_relations']}) rel {s['relation_correct']}/{s['relations']} (len {l['relation_correct']}) FS {s['false_success']} neg {s['negative_correct']} | groups rec {j['group_recovered']}/{j['gold_groups']} prop {j['group_proposed']} indiv {j['individual_credit_errors']} (+{j['individual_positive_credit_errors']}) struct {j['structure_correct']}/{j['structure_scored']} | exec {s['performed_correct']}/{s['performed']} of {s['gold_performed']} | c/doc {c['per_doc_mean']:.1f} med {c['per_doc_median']} prec {c['candidate_precision']}")
    print(json.dumps({g: {k: v for k, v in b.items() if k != "per_thread"} for g, b in boots.items()}))
    print(timing)


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--lock-labels", action="store_true")
    a = p.parse_args()
    lock(a.root) if a.lock_labels else run(a.root)
