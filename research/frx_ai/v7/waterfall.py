"""Experiment A: first point of loss for every gold relation, per pipeline stage.

Runs on stored predictions; never changes them. Stages are ordered as the data
flows, so each relation has one first-failure cause plus all causes it hits.
"""
import argparse
import json
from collections import Counter
from pathlib import Path


def overlap(a, b):
    return max(0, min(a[1], b[1]) - max(a[0], b[0]))


def iou(a, b):
    return overlap(a, b) / max(1, max(a[1], b[1]) - min(a[0], b[0]))


def link(row, pred):
    """Same greedy one-to-one IoU>=.5 mapping as v6 evaluate.mapping."""
    links, used = {}, set()
    for i, e in enumerate(pred["events"]):
        options = [(iou(e["trigger"]["span"], g["trigger"]), j) for j, g in enumerate(row["events"]) if j not in used]
        best = max(options, default=(0, -1))
        if best[0] >= 0.5:
            links[i] = best[1]
            used.add(best[1])
    return links


def owner_state(row, pred, links, j):
    """Why gold event j is or is not available to the pair generator."""
    gold = row["events"][j]
    reverse = {v: k for k, v in links.items()}
    if j in reverse:
        e = pred["events"][reverse[j]]
        return ("ok", e) if e["status"] == "performed" else ("modality:" + e["status"], e)
    touching = [e for e in pred["events"] if overlap(e["trigger"]["span"], gold["trigger"])]
    if touching:
        return "span-boundary", touching[0]
    # Predicted head in the same clause window: a different head of the same action.
    a, b = gold["trigger"]
    near = [e for e in pred["events"] if abs(e["trigger"]["span"][0] - a) < 40]
    return ("other-head", near[0]) if near else ("not-detected", None)


def analyse(row, pred):
    links = link(row, pred)
    detected = pred.get("detected_observations", [])
    out = []
    for g in row.get("observations", []):
        if not g["event_indices"] or g["outcome"] == "unknown":
            continue
        owners = sorted(g["event_indices"])
        states = {j: owner_state(row, pred, links, j) for j in owners}
        matches = [i for i, o in enumerate(detected) if overlap(o["evidence"]["span"], g["span"])]
        right = [i for i in matches if detected[i]["outcome"] == g["outcome"]]
        causes = []
        for j, (s, _) in states.items():
            gs = row["events"][j]["status"]
            if s == "not-detected":
                causes.append("event-not-detected")
            elif s in ("span-boundary", "other-head"):
                causes.append("event-span-or-head-mismatch")
            elif s.startswith("modality:"):
                causes.append("event-modality-excluded:gold-" + gs)
        if not matches:
            causes.append("outcome-not-detected")
        elif not right:
            causes.append("outcome-wrong-class")
        proposals = pred.get("proposals", [])
        def complete(p):
            return sorted(links.get(i, -1) for i in p["event_indices"]) == owners and p["observation"] in right
        proposed = any(complete(p) for p in proposals)
        if proposed:
            # A generator that keeps modality as an attribute did not lose it here.
            causes = [c for c in causes if not c.startswith(("event-modality-excluded", "ok-but-gold"))]
        accepted = any(
            sorted(links.get(i, -1) for i in o["event_indices"]) == owners
            and o["outcome"] == g["outcome"] and overlap(o["evidence"]["span"], g["span"])
            for o in pred.get("observations", [])
        )
        if not causes and not proposed:
            single = len(owners) == 1
            subset = any(
                set(links.get(i, -1) for i in p["event_indices"]) & set(owners) and p["observation"] in right
                for p in proposals
            )
            causes.append("pair-not-proposed" if single else "grouping-incorrect" if subset else "group-not-proposed")
        if not causes and proposed and not accepted:
            causes.append("verifier-rejected")
        identity = [
            j for j, (s, e) in states.items()
            if e is not None and (
                (e.get("identity", {}).get("actor_kind") not in (None, "unknown", row["events"][j].get("actor_kind")))
            )
        ]
        out.append(dict(
            id=row["id"], outcome=g["outcome"], size=len(owners), causes=causes,
            first=causes[0] if causes else "recovered", proposed=proposed, accepted=accepted,
            actor_kind_disagreement=bool(identity), gold_status=[row["events"][j]["status"] for j in owners],
        ))
    return out


def order(c):
    """Earliest pipeline stage wins when one relation has several causes."""
    rank = ["event-not-detected", "event-span-or-head-mismatch", "event-modality-excluded",
            "outcome-not-detected", "outcome-wrong-class", "pair-not-proposed", "group-not-proposed",
            "grouping-incorrect", "verifier-rejected", "recovered"]
    return next((i for i, r in enumerate(rank) if c.startswith(r)), 99)


def summarise(rows, preds):
    records = [r for row, pred in zip(rows, preds) for r in analyse(row, pred)]
    for r in records:
        r["first"] = min(r["causes"] or ["recovered"], key=order)
    n = len(records)
    return dict(
        relations=n,
        proposed=sum(r["proposed"] for r in records),
        recovered=sum(r["accepted"] for r in records),
        first_failure=Counter(r["first"] for r in records).most_common(),
        any_failure=Counter(c for r in records for c in set(c.split(":")[0] for c in r["causes"])).most_common(),
        by_size=Counter((min(r["size"], 2), r["proposed"]) for r in records).most_common(),
        joint=dict(total=sum(r["size"] > 1 for r in records), proposed=sum(r["size"] > 1 and r["proposed"] for r in records)),
        gold_nonperformed_owner=sum(any(s != "performed" for s in r["gold_status"]) for r in records),
    ), records


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--out", type=Path)
    a = p.parse_args()
    rows = [json.loads(x) for x in (a.root / "acceptance-gold.jsonl").read_text().splitlines() if x]
    preds = json.loads((a.root / "final-predictions.json").read_text())
    result = {}
    for name in ["v6-selected", "v5-stanza"]:
        s, records = summarise(rows, preds[name])
        result[name] = dict(summary=s, records=records)
        print(name, json.dumps(s, indent=1, default=str))
    if a.out:
        a.out.write_text(json.dumps(result, indent=1, default=str))
