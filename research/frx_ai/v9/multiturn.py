"""Continuity test: a case grows across turns. Sequences are prepared; responses are never scripted.

Each turn's expectation is a property of the state, so no answer is tied to one example.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import engine  # noqa: E402
import reading as rd  # noqa: E402
import sources  # noqa: E402

SEQUENCES = [
    dict(id="missing-library", problem=dict(game="Example Quest", symptom="The game does not start."), log="missing-dll.log", exists="MSVCP140_ATOMIC_WAIT.dll exists in the prefix system32 folder.", refute="inspect-import-chain"),
    dict(id="undefined-symbol", problem=dict(game="Example Racer", symptom="The game does not launch after the launcher."), log="undefined-symbol.log", exists="libfoo.so.1 is present in the runtime folder.", refute="resolve-library-path"),
    dict(id="page-fault", problem=dict(game="Example Tactics", gpu="NVIDIA", symptom="Random crash after a few minutes."), log="page-fault.log", exists=None, refute="locate-faulting-module"),
    dict(id="device-lost", problem=dict(game="Example Shooter", gpu="AMD Radeon", symptom="Black screen and then the game closes."), log="device-lost.log", exists=None, refute="check-vulkan-support"),
]


def snapshot(state):
    r = engine.report(state)
    return dict(queries=[" ".join(q["terms"]) for q in state.queries], next=[c["id"] for c in r["next_checks"]], hypotheses={h["id"]: h["status"] for h in r["catalogue_hypotheses"]}, findings=[f["term"] for f in r["findings"]][:5], results=list(state.results))


def run(seq, tool, cutoff="2026-10-31"):
    state = engine.State(seq["problem"], cutoff)
    state = engine.investigate(seq["problem"], tool, "iterative", until=cutoff, prior=state)
    t1 = snapshot(state)
    log = rd.logs.read_fixture(seq["log"])
    fresh = engine.update_case(state, log=log)
    state = engine.investigate(seq["problem"], tool, "iterative", until=cutoff, prior=state)
    t2 = snapshot(state)
    new_query_uses_error = any(t.lower() in q.lower() for t in fresh for q in t2["queries"][len(t1["queries"]):])
    checks = dict(t1_asks_for_log="collect-proton-log" in t1["next"][:3], t2_drops_log_request="collect-proton-log" not in t2["next"], t2_searched_the_concrete_error=new_query_uses_error, t2_next_changed=t2["next"] != t1["next"])
    t3 = t2
    if seq["exists"]:
        engine.update_case(state, message=seq["exists"])
        state = engine.investigate(seq["problem"], tool, "iterative", until=cutoff, prior=state)
        t3 = snapshot(state)
        checks["t3_absence_weakened"] = t3["hypotheses"].get("missing-windows-component") in (None, "weakened-by-reported-check")
        checks["t3_conclusion_changed"] = t3["next"] != t2["next"] or t3["hypotheses"] != t2["hypotheses"]
    engine.update_case(state, results=[dict(check=seq["refute"], verdict="refutes", finding="a verificação não confirmou", source="user-reported-check")])
    state = engine.investigate(seq["problem"], tool, "iterative", until=cutoff, prior=state)
    t4 = snapshot(state)
    checks["t4_failure_recorded"] = any(r["check"] == seq["refute"] and r["verdict"] == "refutes" for r in t4["results"])
    checks["t4_not_requested_again"] = seq["refute"] not in t4["next"]
    checks["t4_decision_changed"] = t4["next"] != t3["next"] or t4["hypotheses"] != t3["hypotheses"]
    checks["never_confirmed_a_cause"] = engine.report(state)["confirmed_causes"] == []
    return dict(id=seq["id"], checks=checks, turns=dict(t1=t1, t2=t2, t3=t3, t4=t4))


if __name__ == "__main__":
    tool = sources.Frozen()
    rows = [run(s, tool) for s in SEQUENCES]
    summary = {}
    for r in rows:
        for k, v in r["checks"].items():
            a, b = summary.get(k, (0, 0))
            summary[k] = (a + bool(v), b + 1)
    out = dict(status="synthetic sequences over a frozen real corpus; tests state updates, not diagnosis", summary=summary, rows=rows)
    (engine.net.CACHE / "multiturn.json").write_text(json.dumps(out, indent=1, default=str))
    print(json.dumps(summary, indent=0))
    for r in rows:
        print(r["id"], [(t, r["turns"][t]["next"][:2]) for t in r["turns"]], [q for q in r["turns"]["t4"]["queries"]])
