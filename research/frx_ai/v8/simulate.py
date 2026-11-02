"""Logic test of the investigation loop on synthetic scenarios.

Scenarios and knowledge share one author, so this is circular: it exposes broken loops, such
as repeating a question or ignoring an answer. It says nothing about language or real diagnosis.
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import investigator as inv  # noqa: E402
from logs import read_fixture  # noqa: E402

# Hidden cause, the text shown, the fixture log (None until the check supplies it).
SCENARIOS = [
    ("wrong-library-resolved", "The game does not start after the launcher.", "undefined-symbol.log"),
    ("missing-windows-component", "Game closes right away, nothing on screen.", "missing-dll.log"),
    ("wine-gap", "It crashes when the intro video ends.", "unimplemented.log"),
    ("game-or-wine-fault", "Random crash after a few minutes of play.", "page-fault.log"),
    ("gpu-or-driver-fault", "Black screen and then the game closes.", "device-lost.log"),
    ("prefix-state", "It stopped starting after I moved the library.", "prefix-error.log"),
    ("anti-cheat-support", "Game says it cannot connect to the anti-cheat.", "anticheat.log"),
    ("thread-deadlock", "The game freezes on the loading screen.", "sync-timeout.log"),
    ("unknown-cause", "Something is wrong with the game.", "noise-only.log"),
]
MAX_STEPS = 6


def answer(cause, check):
    """The world: a check tied to the hidden cause supports it, any other refutes it."""
    if check == "collect-proton-log":
        return "log"
    related = check in inv.HYPOTHESES.get(cause, {}).get("checks", [])
    return "supports" if related else "refutes"


def run(policy):
    rows = []
    for cause, text, fixture in SCENARIOS:
        case = dict(game="Synthetic", problem=text, turns=[dict(role="reporter", text=text)], results=[], context={})
        asked, repeated = [], 0
        for _ in range(MAX_STEPS):
            state = inv.state_of(case)
            checks = inv.POLICIES[policy](state)
            if not checks:
                break
            check = checks[0]["id"]
            repeated += check in asked
            asked.append(check)
            reply = answer(cause, check)
            if reply == "log":
                case["log"] = read_fixture(fixture)
            case["results"].append(dict(check=check, verdict="supports" if reply == "log" else reply, finding="simulated", source="simulated"))
        state = inv.state_of(case)
        supported = [h["id"] for h in state["hypotheses"] if h["status"] == "supported-by-reported-check"]
        rows.append(dict(cause=cause, asked=asked, repeated=repeated, cause_among_supported=cause in supported, supported=len(supported), confirmed=len(state["confirmed_causes"])))
    return rows


def summary(rows):
    return dict(scenarios=len(rows), repeated_questions=sum(r["repeated"] for r in rows), cause_among_supported=sum(r["cause_among_supported"] for r in rows), mean_supported=round(sum(r["supported"] for r in rows) / len(rows), 2), confirmed_causes=sum(r["confirmed"] for r in rows), distinct_first_checks=len({r["asked"][0] for r in rows if r["asked"]}))


if __name__ == "__main__":
    import json
    print(json.dumps({p: summary(run(p)) for p in inv.POLICIES}, indent=1))
