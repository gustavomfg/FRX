"""Post-hoc sensitivity of the trajectory result. Written after the first reading; it changes no number.

read-attached-log is a check added in v8, and only the v8 policy can emit it. These readings ask how
much of the advantage depends on it.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import freeze  # noqa: E402
import investigator as inv  # noqa: E402
import trajectory as tr  # noqa: E402

CACHE = freeze.CACHE
READ, LOG = "read-attached-log", "collect-proton-log"


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def scored(rows):
    n = len(rows)
    return dict(decisions=n, top1=[sum(r["top1_ok"] for r in rows), n], top3=[sum(r["top3_ok"] for r in rows), n])


def merged(row):
    """Reading an attached log and asking for one both mean "look at the log"."""
    rename = lambda c: LOG if c == READ else c
    acceptable = {rename(c) for c in row["acceptable"]}
    return dict(row, top1_ok=rename(row["top"]) in acceptable, top3_ok=bool({rename(c) for c in row["top3"]} & acceptable))


def main():
    threads, gold = read(CACHE / "t8-test-reserved.jsonl"), read(CACHE / "t8-test-gold.jsonl")
    rows = {p: [r for t, g in zip(threads, gold) for r in tr.replay(t, g, p, inv.case_from_thread, inv.state_of, inv.POLICIES)] for p in inv.POLICIES}
    out = dict(status="post-hoc on known data; H0 labels")
    out["decisions_with_read_in_acceptable"] = sum(READ in r["acceptable"] for r in rows["discriminate"])
    out["discriminate_top1_is_read"] = sum(r["top"] == READ for r in rows["discriminate"])
    out["without_read_decisions"] = {p: scored([r for r in v if READ not in r["acceptable"]]) for p, v in rows.items()}
    out["log_checks_merged"] = {p: scored([merged(r) for r in v]) for p, v in rows.items()}
    merged_rows = {p: [merged(r) for r in v] for p, v in rows.items()}
    out["log_checks_merged_paired"] = {p: {k: tr.by_thread_difference(merged_rows["discriminate"], merged_rows[p], key=k) for k in ("top1_ok", "top3_ok")} for p in rows if p != "discriminate"}
    top3_sizes = sum(len(r["acceptable"]) for r in rows["discriminate"]) / len(rows["discriminate"])
    out["mean_acceptable_per_decision"] = round(top3_sizes, 2)
    out["catalog_size"] = len(inv.CHECKS)
    (CACHE / "posthoc.json").write_text(json.dumps(out, indent=1))
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
