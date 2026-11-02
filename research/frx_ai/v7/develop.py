"""Development read-out on known data only (P6-dev, and P6 now known)."""
import argparse
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import procedures  # noqa: E402
from bridge import CompactSyntax, read  # noqa: E402
from evaluate import evaluate  # noqa: E402
from waterfall import summarise  # noqa: E402
from classifier import Classifier  # noqa: E402
from l1_adapter import rescore  # noqa: E402

if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--threshold", type=float, default=2.6)
    p.add_argument("--show", type=str, default="")
    a = p.parse_args()
    parser = CompactSyntax(a.root)
    catalog = json.loads((a.root / "catalog.json").read_text())
    for name, file in [("P6-dev", "development-gold-v2.jsonl"), ("P6-known", "acceptance-gold.jsonl")]:
        rows = read(a.root / file)
        t = time.perf_counter()
        preds = [procedures.analyze(r, parser, catalog, a.threshold) for r in rows]
        dt = (time.perf_counter() - t) / len(rows) * 1000
        m = evaluate(rows, preds)
        sel = json.loads((a.root / "selection.json").read_text())
        model = Classifier(a.root / sel["model"])
        lm = evaluate(rows, [rescore(r, x, model, sel["threshold"]) for r, x in zip(rows, preds)])
        print(name, "v7-candidates+L1", dict(correct=lm["relation_correct"], claims=lm["relations"], false_success=lm["false_success"], joint=(lm["joint_correct"], lm["joint_claims"])))
        s, records = summarise(rows, preds)
        pool = sum(len(x["proposals"]) for x in preds)
        print(name, dict(docs=len(rows), proposable=m["proposable_relations"], gold=m["gold_relations"], correct=m["relation_correct"], claims=m["relations"], false_success=m["false_success"], joint_correct=m["joint_correct"], joint_claims=m["joint_claims"], performed=(m["performed_correct"], m["performed"], m["gold_performed"]), cand_per_doc=round(pool / len(rows), 1), ms=round(dt, 1)))
        print(" first", s["first_failure"])
        print(" joint", s["joint"])
        if a.show == name:
            for row, pred in zip(rows, preds):
                t = row["text"]
                gold = [o for o in row.get("observations", []) if o["event_indices"] and o["outcome"] != "unknown"]
                for r, o in zip([x for x in records if x["id"] == row["id"]], gold):
                    if r["first"] != "recovered":
                        print("MISS", r["first"], row["id"], repr(t[o["span"][0]:o["span"][1]])[:60], [t[slice(*row["events"][j]["trigger"])] for j in o["event_indices"]], "|", repr(t[:110]))
                from waterfall import link
                links = link(row, pred)
                for c in pred["observations"]:
                    own = sorted(links.get(i, -1) for i in c["event_indices"])
                    ok = any(g["outcome"] == c["outcome"] and sorted(g["event_indices"]) == own and min(c["evidence"]["span"][1], g["span"][1]) > max(c["evidence"]["span"][0], g["span"][0]) for g in gold)
                    if not ok:
                        print("WRONG", c["outcome"], row["id"], repr(c["evidence"]["text"])[:60], [pred["events"][i]["trigger"]["text"] for i in c["event_indices"]], round(c["score"], 2), "|", repr(t[:110]))
