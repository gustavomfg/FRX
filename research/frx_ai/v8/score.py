"""Scoring of outcome detection and procedure groups, on any gold of the guide's shape.

Comparators map to the same unit reading. v7 has one positive class, so cross-system class
agreement merges resolved and improved; v8's finer split is reported apart.
"""
import random

POSITIVE = {"resolved", "improved"}
CLAIM = POSITIVE | {"not_resolved", "worsened", "mixed"}


def merged(cls):
    return "positive" if cls in POSITIVE else cls


def wilson(k, n, z=1.96):
    if not n:
        return None
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    m = z * (p * (1 - p) / n + z * z / (4 * n * n)) ** .5
    return [round((c - m) / d, 3), round((c + m) / d, 3)]


def outcome_metrics(gold, pred):
    """Both are lists of unit readings in the same order. "unclear" gold is excluded from errors."""
    rows = [(g["overall"], p) for g, p in zip(gold, pred) if g["overall"] != "unclear"]
    claims = [x for x in rows if x[0] in CLAIM]
    said = [x for x in rows if x[1] in CLAIM]
    said_positive = [x for x in rows if x[1] in POSITIVE or x[1] == "positive"]
    false_positive = [x for x in said_positive if x[0] not in POSITIVE and x[0] != "mixed"]
    hit = [x for x in claims if x[1] in CLAIM or x[1] == "positive"]
    return dict(
        units=len(rows), gold_claims=len(claims), system_claims=len(said) + sum(x[1] == "positive" for x in rows),
        claim_recall=[len(hit), len(claims)], claim_recall_ci=wilson(len(hit), len(claims)),
        claim_precision=[len([x for x in said + [y for y in rows if y[1] == "positive"] if x[0] in CLAIM]), len(said) + sum(x[1] == "positive" for x in rows)],
        positive_claims=len(said_positive), false_positive=len(false_positive),
        positive_precision=[len(said_positive) - len(false_positive), len(said_positive)],
        positive_precision_ci=wilson(len(said_positive) - len(false_positive), len(said_positive)),
        merged_class_correct=[sum(merged(g) == merged(p) for g, p in rows), len(rows)],
    )


def fine_metrics(gold, pred):
    rows = [(g["overall"], p) for g, p in zip(gold, pred) if g["overall"] != "unclear"]
    resolved = [x for x in rows if x[1] == "resolved"]
    false_resolved = [x for x in resolved if x[0] not in ("resolved", "improved", "mixed")]
    exact = sum(g == p for g, p in rows)
    return dict(exact_class=[exact, len(rows)], resolved_claims=len(resolved), false_resolved=len(false_resolved),
                strict_false_resolved=len([x for x in resolved if x[0] != "resolved"]),
                resolved_precision=[len(resolved) - len(false_resolved), len(resolved)], resolved_precision_ci=wilson(len(resolved) - len(false_resolved), len(resolved)))


def paired_bootstrap(gold, a, b, n=5000, seed="frx-v8"):
    """CI of accuracy(a) - accuracy(b) on merged classes. One unit per thread by design."""
    rows = [(merged(g["overall"]), merged(x), merged(y)) for g, x, y in zip(gold, a, b) if g["overall"] != "unclear"]
    rng = random.Random(seed)
    diffs = []
    for _ in range(n):
        s = [rows[rng.randrange(len(rows))] for _ in rows]
        diffs.append(sum((g == x) - (g == y) for g, x, y in s) / len(s))
    diffs.sort()
    return dict(difference=round(sum((g == x) - (g == y) for g, x, y in rows) / len(rows), 3), ci95=[round(diffs[int(.025 * n)], 3), round(diffs[int(.975 * n)], 3)])


def _overlaps(span, quote_spans):
    return any(span[0] < b and a < span[1] for a, b in quote_spans)


def group_correct(text, gold, relation, items):
    """A gold group is recovered when the relation matches and every gold action is covered,
    with no more than one extra proposed member."""
    if gold["relation"] not in ("joint", "alternatives", "sequence"):
        return None
    if relation != gold["relation"]:
        return False
    spans = []
    for q in gold["actions"]:
        i = text.find(q)
        if i >= 0:
            spans.append((i, i + len(q)))
    covered = all(any(_overlaps(it["span"], [s]) for it in items) for s in spans)
    return covered and len(items) <= len(spans) + 1


def group_metrics(rows):
    """rows: dicts of text, gold, relation, items."""
    groups = [r for r in rows if r["gold"]["relation"] in ("joint", "alternatives", "sequence")]
    correct = sum(bool(group_correct(r["text"], r["gold"], r["relation"], r["items"])) for r in groups)
    relation_ok = sum(r["relation"] == r["gold"]["relation"] for r in groups)
    invented = sum(r["relation"] in ("joint", "alternatives", "sequence") and r["gold"]["relation"] in ("single", "not_applicable") for r in rows)
    return dict(gold_groups=len(groups), groups_correct=correct, relation_correct=relation_ok, invented_groups=invented, per_type={k: [sum(bool(group_correct(r["text"], r["gold"], r["relation"], r["items"])) for r in groups if r["gold"]["relation"] == k), sum(r["gold"]["relation"] == k for r in groups)] for k in ("joint", "alternatives", "sequence")})
