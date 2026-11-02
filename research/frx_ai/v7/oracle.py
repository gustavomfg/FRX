"""Experiment B: verifier ceiling with gold candidates. DIAGNOSTIC ONLY.

Gold spans and owners enter here and nowhere else; inference modules must not
import this file (test_contracts enforces it).
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import procedures  # noqa: E402
import events as _v5_events  # noqa: E402
from bridge import CompactSyntax, evidence, read  # noqa: E402
from classifier import Classifier  # noqa: E402
from grounding import game_mentions, identity, masked  # noqa: E402
from pairs import deterministic_score, verify_literal  # noqa: E402
from l1_adapter import features  # noqa: E402


_inherited_evidence = _v5_events.evidence
_v5_events.evidence = lambda text, a, b: _inherited_evidence(text, min(a, b), max(a, b))


def node_at(parsed, span):
    a, b = span
    hits = [w for w in parsed["words"] if a <= w["span"][0] < b or w["span"][0] <= a < w["span"][1]]
    verbs = [w for w in hits if w["pos"] in ("VERB", "AUX")]
    return (verbs or hits or [None])[0]


def gold_bundle(row, parser, catalog):
    text = row["text"]
    parsed = parser.parse(masked(text))
    found = game_mentions(text, catalog)
    events, map_event = [], {}
    for j, g in enumerate(row["events"]):
        w = node_at(parsed, g["trigger"])
        if w is None:
            continue
        map_event[j] = len(events)
        a, b = g["trigger"]
        events.append(dict(trigger=evidence(text, a, b), status=g["status"], node=w["i"], sentence=w["sentence"], identity=identity(text, parsed, w["i"], row, found), lemma=w["lemma"]))
    observed = []
    for g in row.get("observations", []):
        w = node_at(parsed, g["span"])
        if w is None:
            observed.append(None)
            continue
        a, b = g["span"]
        observed.append(dict(evidence=evidence(text, a, b), outcome=g["outcome"], node=w["i"], sentence=w["sentence"], identity=identity(text, parsed, w["i"], row, found), gold=g))
    observed_clean = [o for o in observed if o]
    return dict(events=events, observations=observed_clean, parsed=parsed, game_mentions=found, temporal=[]), map_event


def options_for(row, b, map_event, oi):
    """Gold group plus each single gold event as distractors."""
    g = b["observations"][oi]["gold"]
    gold = tuple(sorted(map_event[j] for j in g["event_indices"] if j in map_event))
    singles = {(i,) for i in range(len(b["events"]))}
    return gold, sorted(singles | ({gold} if gold else set()))


def run(root, verifier_path, threshold, ignore_literal=False, scorer="L1", gold="acceptance-gold.jsonl", prefix=""):
    parser = CompactSyntax(root)
    catalog = json.loads((root / "catalog.json").read_text())
    model = Classifier(verifier_path)
    rows = [r for r in read(root / gold) if r["id"].startswith(prefix)]
    n = dict(relations=0, group_available=0, chosen_correct=0, accepted=0, accepted_correct=0, rejected_literal=0, rejected_threshold=0, wrong_group=0, false_success=0, joint=0, joint_correct=0, no_owner_results=0, no_owner_false_accept=0)
    for row in rows:
        b, map_event = gold_bundle(row, parser, catalog)
        words = b["parsed"]["words"]
        if not b["events"]:
            continue
        for oi, o in enumerate(b["observations"]):
            g = o["gold"]
            relevant = bool(g["event_indices"]) and g["outcome"] != "unknown"
            if not relevant and g["outcome"] == "unknown":
                continue
            gold, opts = options_for(row, b, map_event, oi)
            if relevant:
                n["relations"] += 1
                n["joint"] += len(g["event_indices"]) > 1
                n["group_available"] += len(gold) == len(g["event_indices"])
            else:
                n["no_owner_results"] += 1
            pool = [features(row["text"], b, group, oi) for group in opts]
            kept = []
            for p in pool:
                verdict, reason = verify_literal(row["text"], b, p)
                if verdict == "candidate" or ignore_literal:
                    kept.append(p)
            if relevant and not any(tuple(sorted(p["event_indices"])) == gold for p in kept):
                n["rejected_literal"] += 1
            if not kept:
                continue
            if scorer == "L1":
                scores = model(row, b, kept)
            elif scorer == "structural-v6":
                scores = [deterministic_score(p) for p in kept]
            else:
                scores = [procedures.structural_score(row["text"], p["event_indices"], o, b["events"], words, b["observations"]) for p in kept]
            order = sorted(range(len(kept)), key=lambda i: (-scores[i], -len(kept[i]["event_indices"])))
            best = kept[order[0]]
            chose = tuple(sorted(best["event_indices"]))
            if relevant:
                n["chosen_correct"] += chose == gold
            if scores[order[0]] >= threshold:
                n["accepted"] += 1
                ok = relevant and chose == gold
                n["accepted_correct"] += ok
                if relevant:
                    n["joint_correct"] += ok and len(gold) > 1
                else:
                    n["no_owner_false_accept"] += 1
                if not ok and g["outcome"] == "improved":
                    n["false_success"] += 1
                if relevant and not ok:
                    n["wrong_group"] += 1
            elif relevant:
                n["rejected_threshold"] += 1
    return n


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--out", type=Path)
    p.add_argument("--gold", default="acceptance-gold.jsonl")
    p.add_argument("--prefix", default="")
    a = p.parse_args()
    sel = json.loads((a.root / "selection.json").read_text())
    path = a.root / sel["model"]
    result = {}
    for label, kw in [
        ("L1-with-literal-filter", dict(threshold=sel["threshold"])),
        ("L1-no-literal-filter", dict(threshold=sel["threshold"], ignore_literal=True)),
        ("structural-v6-literal", dict(threshold=0.5, scorer="structural-v6")),
        ("structural-v6-no-literal", dict(threshold=0.5, scorer="structural-v6", ignore_literal=True)),
        ("structural-v7-no-literal", dict(threshold=3.4, scorer="structural-v7", ignore_literal=True)),
    ]:
        result[label] = run(a.root, path, gold=a.gold, prefix=a.prefix, **kw)
        print(label, json.dumps(result[label]))
    if a.out:
        a.out.write_text(json.dumps(result, indent=1))
