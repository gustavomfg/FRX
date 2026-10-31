"""Post-test error reading. Publishes findings; nothing here feeds back into v7."""
import json
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from waterfall import link, overlap  # noqa: E402

root = Path(sys.argv[1])
rows = [json.loads(x) for f in ["p7-gold.jsonl", "j7-gold.jsonl"] for x in (root / f).read_text().splitlines()]
preds = json.loads((root / "final-predictions.json").read_text())["v7"]
results = json.loads((root / "final-results.json").read_text())
out = dict(first_failure={g: {k: results["results"][g][k]["first_failure"] for k in ["v7", "v6-selected"]} for g in results["results"]}, false_success=[], groups=[])
shapes = Counter()
for row, pred in zip(rows, preds):
    t = row["text"]
    links = link(row, pred)
    gold = [o for o in row["observations"] if o["event_indices"] and o["outcome"] != "unknown"]
    for c in pred["observations"]:
        own = sorted(links.get(i, -1) for i in c["event_indices"])
        ok = any(g["outcome"] == c["outcome"] and sorted(g["event_indices"]) == own and overlap(c["evidence"]["span"], g["span"]) for g in gold)
        shapes[(c["structure"], c["attribution"]["kind"], ok)] += 1
        if not ok and c["outcome"] == "improved":
            out["false_success"].append(dict(id=row["id"], url=row["url"], result=c["evidence"]["text"][:70], owners=[pred["events"][i]["trigger"]["text"] for i in c["event_indices"]], structure=c["structure"], gold_results=[t[slice(*g["span"])][:50] for g in gold][:3]))
    for g in gold:
        if len(g["event_indices"]) < 2:
            continue
        detected = pred["detected_observations"]
        cands = [p for p in pred["proposals"] if overlap(detected[p["observation"]]["evidence"]["span"], g["span"])]
        found = [j in {links.get(i) for i in range(len(pred["events"]))} for j in g["event_indices"]]
        out["groups"].append(dict(id=row["id"], url=row["url"], structure=g["structure"], owners=[t[slice(*row["events"][j]["trigger"])] for j in g["event_indices"]], owners_detected=sum(found), result_detected=bool(cands), proposed=[[pred["events"][i]["trigger"]["text"] for i in p["event_indices"]] for p in cands][:3]))
out["claim_shapes"] = [[*k, v] for k, v in shapes.most_common()]
(root / "posthoc.json").write_text(json.dumps(out, indent=1, ensure_ascii=False))
print(json.dumps(out, indent=0, ensure_ascii=False))
