"""Frozen comparators on one cohort, each in its own process because lab module names clash.

v7: explicit rules over a pretrained parser. v2-weak: regex votes. v2-e07: learned weights, frozen.
Nothing is fitted here. Output goes to the cache as unit readings.
"""
import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LAB = HERE.parent
CACHE = Path.home() / ".cache/frx-ai-v8-20261030"
V7 = Path.home() / ".cache/frx-ai-rnd-v7-20261030"
V2 = Path.home() / ".cache/frx-ai-rnd-v2-20261029"


def units(rows):
    """One unit reading from a set of directions."""
    kinds = set(rows)
    positive, negative = "positive" in kinds, bool(kinds & {"not_resolved", "worsened"})
    if positive and negative or "mixed" in kinds:
        return "mixed"
    if positive:
        return "positive"
    if "worsened" in kinds:
        return "worsened"
    return "not_resolved" if "not_resolved" in kinds else "no_claim"


def app_of(title):
    m = re.search(r"\((\d+)\)\s*$", title or "")
    # The v2 contract requires a Steam AppID; threads without one get a stable placeholder.
    return m.group(1) if m else "10"


def run_v7(rows):
    sys.path.insert(0, str(LAB / "v7"))
    import procedures
    from bridge import CompactSyntax
    parser = CompactSyntax(V7)
    catalog = json.loads((V7 / "catalog.json").read_text())
    mapping = dict(improved="positive", persistent="not_resolved", regression="worsened")
    out = []
    for r in rows:
        row = dict(id=r["id"], text=r["text"], source_text=r["text"], source_offset=0, title=r["title"], app_id=app_of(r["title"]), source_thread_app_id=app_of(r["title"]), thread=r["thread"], url=r["url"], author_association=r.get("author_association"), reporter_id="unknown", document_id=r["id"])
        try:
            p = procedures.analyze(row, parser, catalog)
        except ValueError as e:
            out.append(dict(id=r["id"], overall="no_claim", relation="not_applicable", items=[], failure=str(e)[:80]))
            continue
        groups = [o for o in p["observations"] if len(o.get("event_indices", [])) > 1]
        chosen = groups[0] if groups else (p["observations"][0] if p["observations"] else None)
        relation, items = "not_applicable", []
        if chosen:
            relation = chosen.get("structure", "single")
            items = [dict(span=list(p["events"][i]["trigger"]["span"]), text=p["events"][i]["trigger"]["quote"] if "quote" in p["events"][i]["trigger"] else "") for i in chosen["event_indices"]]
        out.append(dict(id=r["id"], overall=units([mapping.get(o["outcome"], o["outcome"]) for o in p["detected_observations"]]), relation=relation, items=items))
    return out


def run_weak(rows):
    sys.path.insert(0, str(LAB / "v2"))
    from weak import vote
    mapping = {"positive": "positive", "not-resolved": "not_resolved", "worsened": "worsened", "unknown": "no_claim", None: "no_claim"}
    return [dict(id=r["id"], overall=mapping[vote(dict(text=r["text"], action_span=[0, len(r["text"])]))]) for r in rows]


def run_e07(rows):
    """The selected v2 relation weights: selective candidates only, as that lab decided."""
    sys.path.insert(0, str(LAB / "v2"))
    from runtime import Runtime
    runtime = Runtime(V2 / "runtime-relation")
    mapping = {"positive": "positive", "not-resolved": "not_resolved", "worsened": "worsened", "mixed": "mixed"}
    out = []
    for r in rows:
        request = dict(context=dict(app_id=app_of(r["title"])), evidence=[dict(text=r["text"], url=r["url"], app_id=app_of(r["title"]), game=r["title"], published_at="2020-01-01T00:00:00Z")])
        try:
            result = runtime.analyze(request)
        except (ValueError, KeyError) as e:
            out.append(dict(id=r["id"], overall="no_claim", failure=str(e)[:80]))
            continue
        kinds = [mapping[g["outcome_candidate"]] for g in result["candidates"] if g.get("selective_candidate") and g.get("outcome_candidate") in mapping]
        everything = [mapping[g["outcome_candidate"]] for g in result["candidates"] if g.get("outcome_candidate") in mapping]
        out.append(dict(id=r["id"], overall=units(kinds), overall_any=units(everything), candidates=len(result["candidates"]), rejected=result.get("rejected") or None))
    return out


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("system", choices=["v7", "weak", "e07"])
    p.add_argument("cohort")
    a = p.parse_args()
    rows = [json.loads(x) for x in (CACHE / f"{a.cohort}-reserved.jsonl").read_text().splitlines()]
    out = dict(v7=run_v7, weak=run_weak, e07=run_e07)[a.system](rows)
    (CACHE / f"{a.cohort}-pred-{a.system}.json").write_text(json.dumps(out, indent=1, default=str))
    print(a.system, len(out), "failures", sum("failure" in x for x in out))
