"""Full diagnostic trajectories: schema, validation and incremental replay.

A replay shows a policy only what had been said before each decision. It scores decisions;
it does not establish that any check would have found the cause.
"""
from collections import Counter

STEP_KINDS = ("problem", "evidence", "hypothesis", "verification", "result", "decision")
ACTOR = ("reporter", "maintainer", "other")


def validate(gold, thread, catalog):
    """Every step quotes its turn literally, and every decision uses known verifications."""
    errors = []
    turns = [thread["body"]] + [c["text"] for c in thread["comments"]]
    for i, step in enumerate(gold.get("steps", [])):
        if step.get("kind") not in STEP_KINDS:
            errors.append(f"step {i}: kind")
        turn = step.get("turn")
        if not isinstance(turn, int) or not 0 <= turn < len(turns):
            errors.append(f"step {i}: turn")
        elif step.get("quote") and step["quote"] not in turns[turn]:
            errors.append(f"step {i}: quote is not literal: {step['quote'][:40]!r}")
    for d in gold.get("decisions", []):
        if not 0 <= d["after_turn"] < len(turns):
            errors.append(f"decision after {d['after_turn']}")
        for check in d["acceptable"] + d.get("already_known", []):
            if check not in catalog:
                errors.append(f"unknown check {check}")
    return errors


def replay(thread, gold, policy, case_from_thread, state_of, policies):
    """One reading per decision point. Later turns stay invisible."""
    out = []
    previous = None
    for d in gold["decisions"]:
        comments = thread["comments"][:d["after_turn"]]
        state = state_of(case_from_thread(thread["title"], thread["body"], comments))
        checks = [c["id"] for c in policies[policy](state)]
        top = checks[0] if checks else None
        out.append(dict(thread=thread["id"], after_turn=d["after_turn"], top=top, top3=checks[:3], acceptable=d["acceptable"], maintainer=d.get("maintainer_asked"),
                        top1_ok=top in d["acceptable"], top3_ok=bool(set(checks[:3]) & set(d["acceptable"])), redundant=top in d.get("already_known", []),
                        changed=previous is not None and top != previous, should_change=d.get("should_change"), unsupported_cause=bool(state["confirmed_causes"]),
                        changes_state_first=bool(top and top in ("reset-prefix-with-backup", "test-other-proton", "bisect-proton-versions"))))
        previous = top
    return out


def summarise(rows):
    n = len(rows)
    updates = [r for r in rows if r["should_change"] is not None]
    return dict(decisions=n, threads=len({r["thread"] for r in rows}), top1=[sum(r["top1_ok"] for r in rows), n], top3=[sum(r["top3_ok"] for r in rows), n], redundant=[sum(r["redundant"] for r in rows), n],
                update_correct=[sum(r["changed"] == r["should_change"] for r in updates), len(updates)], unsupported_cause=sum(r["unsupported_cause"] for r in rows), changes_state_first=sum(r["changes_state_first"] for r in rows),
                top_counter=Counter(r["top"] for r in rows).most_common(5))


def by_thread_difference(rows_a, rows_b, key="top3_ok", n=5000, seed="frx-v8-t"):
    """Paired thread bootstrap of a - b on one binary score."""
    import random
    threads = sorted({r["thread"] for r in rows_a})
    per = {t: (sum(r[key] for r in rows_a if r["thread"] == t), sum(r[key] for r in rows_b if r["thread"] == t), sum(r["thread"] == t for r in rows_a)) for t in threads}
    rng = random.Random(seed)
    diffs = []
    for _ in range(n):
        s = [per[threads[rng.randrange(len(threads))]] for _ in threads]
        total = sum(x[2] for x in s) or 1
        diffs.append(sum(x[0] - x[1] for x in s) / total)
    diffs.sort()
    total = sum(x[2] for x in per.values()) or 1
    return dict(difference=round(sum(x[0] - x[1] for x in per.values()) / total, 3), ci95=[round(diffs[int(.025 * n)], 3), round(diffs[int(.975 * n)], 3)])


def step_recall(thread, gold, results):
    """How many annotated results the state extraction found, by turn and direction."""
    wanted = [s for s in gold.get("steps", []) if s["kind"] == "result"]
    positive = {"resolved", "improved"}

    def direction(c):
        return "positive" if c in positive else "negative" if c in ("not_resolved", "worsened") else c

    hit = 0
    for step in wanted:
        hit += any(r["turn"] == step["turn"] and direction(r["cls"]) == direction(step.get("cls")) for r in results)
    return hit, len(wanted), len(results)
