"""First and only reading of the v8 test cohorts. H0 labels: never human validation."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import detector  # noqa: E402
import freeze  # noqa: E402
import investigator as inv  # noqa: E402
import joint  # noqa: E402
import logs  # noqa: E402
import score  # noqa: E402
import trajectory as tr  # noqa: E402

CACHE = freeze.CACHE
VENV = Path.home() / ".cache/frx-ai-rnd-v6-20261030/venv/bin/python"
STATUS = "H0: labels by the agent, not human validation"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def lock():
    dest = CACHE / "label-lock.json"
    if dest.exists():
        raise SystemExit("labels already locked")
    files = ["o8-test-gold.jsonl", "t8-test-gold.jsonl", "l8-gold.jsonl"]
    dest.write_text(json.dumps({f: sha(CACHE / f) for f in files}, indent=1))
    print("locked", files)


def checks():
    bad = freeze.changed()
    if bad:
        raise SystemExit(f"frozen code changed: {bad}")
    state = json.loads((CACHE / "label-lock.json").read_text())
    for f, h in state.items():
        if sha(CACHE / f) != h:
            raise SystemExit(f"labels changed after the lock: {f}")


def comparators(cohort):
    for system in ("v7", "weak", "e07"):
        file = CACHE / f"{cohort}-pred-{system}.json"
        if not file.exists():
            subprocess.run([str(VENV), "-B", str(HERE / "compare.py"), system, cohort], check=True, env={"HF_HUB_OFFLINE": "1", "PATH": "/usr/bin:/bin", "HOME": str(Path.home())})


def read_outcomes(cohort):
    rows, gold = read(CACHE / f"{cohort}-reserved.jsonl"), read(CACHE / f"{cohort}-gold.jsonl")
    comparators(cohort)
    found = [detector.detect(r["text"]) for r in rows]
    preds = dict(v8=[p["overall"] for p in found], no_claim=["no_claim"] * len(rows))
    others = {s: json.loads((CACHE / f"{cohort}-pred-{s}.json").read_text()) for s in ("v7", "weak", "e07")}
    preds.update({s: [x["overall"] for x in v] for s, v in others.items()})
    preds["e07_any"] = [x.get("overall_any", "no_claim") for x in others["e07"]]
    result = {k: score.outcome_metrics(gold, v) for k, v in preds.items()}
    result["v8"]["fine"] = score.fine_metrics(gold, preds["v8"])
    pairs = {s: score.paired_bootstrap(gold, preds["v8"], preds[s]) for s in ("v7", "weak", "e07", "e07_any", "no_claim")}
    groups_v8, groups_v7 = [], []
    for r, g, p, o in zip(rows, gold, found, others["v7"]):
        relation, items = joint.unit_relation(r["text"], p)
        groups_v8.append(dict(text=r["text"], gold=g, relation=relation, items=items))
        groups_v7.append(dict(text=r["text"], gold=g, relation=o.get("relation", "not_applicable"), items=o.get("items", [])))
    credit = sum(proc["attribution"] == "single-reported" and proc["relation"] != "single" for r in rows for proc in joint.procedures(r["text"]))
    details = [dict(id=r["id"], gold=g["overall"], v8=p["overall"], v7=o["overall"], e07=e["overall"], gold_relation=g["relation"], v8_relation=x["relation"], v7_relation=y["relation"], quotes=[c["cls"] + ": " + c["quote"][:60] for c in p["claims"]][:5])
               for r, g, p, o, e, x, y in zip(rows, gold, found, others["v7"], others["e07"], groups_v8, groups_v7)]
    return dict(status=STATUS, units=len(rows), outcome=result, paired_v8_minus=pairs, groups=dict(v8=score.group_metrics(groups_v8), v7=score.group_metrics(groups_v7)), false_individual_credit_v8=credit, details=details,
                failures=dict((s, sum("failure" in x for x in v)) for s, v in others.items()))


def read_trajectories(cohort):
    threads, gold = read(CACHE / f"{cohort}-reserved.jsonl"), read(CACHE / f"{cohort}-gold.jsonl")
    errors = {t["id"]: tr.validate(g, t, set(inv.CHECKS)) for t, g in zip(threads, gold)}
    if any(errors.values()):
        raise SystemExit(str({k: v for k, v in errors.items() if v}))
    rows = {}
    for policy in inv.POLICIES:
        rows[policy] = []
        for t, g in zip(threads, gold):
            rows[policy] += tr.replay(t, g, policy, inv.case_from_thread, inv.state_of, inv.POLICIES)
    summary = {p: tr.summarise(v) for p, v in rows.items()}
    pairs = {p: {k: tr.by_thread_difference(rows["discriminate"], rows[p], key=k) for k in ("top3_ok", "top1_ok")} for p in rows if p != "discriminate"}
    found = wanted = said = 0
    for t, g in zip(threads, gold):
        a, b, c = tr.step_recall(t, g, inv.extract_results(inv.case_from_thread(t["title"], t["body"], t["comments"])["turns"]))
        found += a
        wanted += b
        said += c
    kinds = {}
    for g in gold:
        for s in g["steps"]:
            kinds[s["kind"]] = kinds.get(s["kind"], 0) + 1
    return dict(status=STATUS, threads=len(threads), summary=summary, paired_discriminate_minus=pairs, result_steps=dict(found=found, annotated=wanted, extracted=said), step_kinds=kinds, details=rows["discriminate"], maintainers=[(r["thread"], r["after_turn"], r["maintainer"]) for r in rows["discriminate"]])


def read_logs():
    rows, gold = read(CACHE / "l8-reserved.jsonl"), {g["id"]: g for g in read(CACHE / "l8-gold.jsonl")}
    out = []
    for r in rows:
        g = gold[r["id"]]
        excerpt = logs.inline_excerpts(r["text"])
        reading = logs.read_log(excerpt)
        lines = logs.literal_search(excerpt)
        kinds = {s["kind"] for s in reading["signals"]}
        wanted = set(g["signals"])
        out.append(dict(id=r["id"], gold=sorted(wanted), kinds=sorted(kinds), found=len(kinds & wanted), wanted=len(wanted), extra=len(kinds - wanted), literal_lines=len(lines), total_lines=len(excerpt.splitlines()), literal_hit=sum(any(q.lower() in x.lower() for x in lines) for q in g["quotes"]), rule_hit=sum(any(q.lower() in s["quote"].lower() for s in reading["signals"]) for q in g["quotes"]), quotes=len(g["quotes"]), modules=reading["modules"], gold_module=g.get("module"), cause=reading["confirmed_cause"]))
    fixtures = {}
    expected = json.loads((logs.FIXTURES / "expected.json").read_text())
    for name, want in expected.items():
        reading = logs.read_log(logs.read_fixture(name))
        fixtures[name] = sorted({s["kind"] for s in reading["signals"]}) == sorted(want)
    return dict(status=STATUS, excerpts=len(rows), signals_found=[sum(x["found"] for x in out), sum(x["wanted"] for x in out)], extra_signals=sum(x["extra"] for x in out), literal_quotes=[sum(x["literal_hit"] for x in out), sum(x["quotes"] for x in out)], rule_quotes=[sum(x["rule_hit"] for x in out), sum(x["quotes"] for x in out)], lines_shown_literal=sum(x["literal_lines"] for x in out), lines_total=sum(x["total_lines"] for x in out), invented_causes=sum(x["cause"] is not None for x in out), modules_correct=[sum(bool(x["gold_module"]) and x["gold_module"] in x["modules"] for x in out), sum(bool(x["gold_module"]) for x in out)], fixtures_ok=[sum(fixtures.values()), len(fixtures)], details=out)


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("command", choices=["lock", "first", "dev"])
    a = p.parse_args()
    if a.command == "lock":
        lock()
    elif a.command == "dev":
        print(json.dumps(dict(o=read_outcomes("o8-dev"), t=read_trajectories("t8-dev")), indent=1, default=str)[:6000])
    else:
        checks()
        dest = CACHE / "first-reading.json"
        if dest.exists():
            raise SystemExit("preserve the first reading; later changes are known replays")
        out = dict(outcomes=read_outcomes("o8-test"), trajectories=read_trajectories("t8-test"), logs=read_logs(), frozen=json.loads((CACHE / "frozen.json").read_text())["frozen_at"])
        with dest.open("x") as f:
            json.dump(out, f, indent=1, default=str)
        print("first reading stored")
