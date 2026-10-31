"""Frozen v6 L1 weights over v7 candidates, as a comparator. No refit, no new labels."""
from classifier import Classifier
from grounding import ancestors
from pairs import conflict
import re
from bridge import evidence


def features(text, b, group, oi):
    """The exact 19-feature formula of v6 pairs.candidates, on an explicit group."""
    events, words, obs = b["events"], b["parsed"]["words"], b["observations"][oi]
    a, c = obs["evidence"]["span"]
    chosen = [events[i] for i in group]
    left = min([a] + [e["trigger"]["span"][0] for e in chosen])
    right = max([c] + [e["trigger"]["span"][1] for e in chosen])
    span = text[left:right]
    games, actors = [], []
    for e in chosen:
        g, h = conflict(e["identity"], obs["identity"])
        games.append(g)
        actors.append(h)
    deps = sum(any(p["i"] == obs["node"] for p in ancestors(words, e["node"])[1:]) for e in chosen) / len(chosen)
    before = sum(e["trigger"]["span"][0] < a for e in chosen) / len(chosen)
    delta = chosen[0]["sentence"] - obs["sentence"]
    v = [
        min(abs(e["trigger"]["span"][0] - a) for e in chosen) / 650, float(delta == 0), delta, before, len(group) / 6, deps,
        float(bool(re.search(r"(?i)\b(?:after|by|since)\b", span))), float(bool(re.search(r"(?i)\bbefore\b", span))), 0.0,
        float(any(games)), float(any(actors)),
        float(all(e["identity"]["game_app_id"] for e in chosen) and obs["identity"]["game_app_id"] is not None),
        float(all(e["identity"]["actor_kind"] == "reporter" for e in chosen)),
        float(obs["outcome"] == "persistent"), float(obs["outcome"] == "improved"), float(obs["outcome"] == "regression"),
        float(any(left < o["evidence"]["span"][0] < right and o is not obs for o in b["observations"])), float(deps > 0),
        float(any(e["identity"]["game_app_id"] is None for e in chosen)),
    ]
    return dict(observation=oi, event_indices=list(group), features=v, hard_conflict=bool(any(games) or any(actors)), incomplete_joint=False, evidence=evidence(text, left, min(right, left + 1300)))


def rescore(row, pred, model, threshold):
    """Replace the v7 decision by L1 over the same candidate pool."""
    text = row["text"]
    b = dict(events=pred["events"], observations=pred["detected_observations"], parsed=pred["parsed"])
    by = {}
    for p in pred["proposals"]:
        by.setdefault(p["observation"], []).append(p)
    out = []
    for oi, ps in by.items():
        pool = [features(text, b, p["event_indices"], oi) for p in ps]
        scores = model(row, b, pool)
        i = max(range(len(pool)), key=lambda k: (scores[k], len(pool[k]["event_indices"])))
        if scores[i] >= threshold and not pool[i]["hard_conflict"]:
            out.append(dict(pred["detected_observations"][oi], event_indices=pool[i]["event_indices"], joint=len(pool[i]["event_indices"]) > 1, relation="v6-L1-over-v7-candidates", score=float(scores[i]), individual_cause="unknown", causality_confirmed=False, execution_authorized=False, automatic_approval=False))
    return dict({k: v for k, v in pred.items()}, observations=out)
