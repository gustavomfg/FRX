"""Publish offsets, provenance and metrics, never source bodies or weights."""
import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
STRIP = {"text", "source_text", "quote", "problem", "maintainer_text", "query", "whole", "result", "gold_results", "proposed", "owners", "directory", "finding"}


def clean(value):
    if isinstance(value, dict):
        return {k: clean(v) for k, v in value.items() if k not in STRIP}
    if isinstance(value, list):
        return [clean(v) for v in value]
    return value


def read(p):
    return [json.loads(x) for x in Path(p).read_text().splitlines() if x.strip()]


def publish(root):
    dest = HERE / "snapshot"
    dest.mkdir(exist_ok=True)
    for name in ["p7-gold", "j7-gold", "c7-cases"]:
        (dest / f"{name}.json").write_text(json.dumps(clean(read(root / f"{name}.jsonl")), indent=1, ensure_ascii=False))
    for name in ["reservation", "frozen", "label-lock", "final-results", "waterfall-p6", "oracle-p6", "oracle-p7m", "oracle-j7", "selector-cv", "selector-selection", "selector-final", "c7-gold", "c7-label-lock", "c7-results", "c7-known-replay", "investigator-frozen", "posthoc", "efficiency-language", "efficiency-cli"]:
        file = root / f"{name}.json"
        if file.exists():
            (dest / file.name).write_text(json.dumps(clean(json.loads(file.read_text())), indent=1, ensure_ascii=False))
    rows = read(root / "d7.jsonl")
    counts = {}
    for r in rows:
        counts.setdefault(r["split"], {})
        for x in r["labels"]:
            counts[r["split"]][x] = counts[r["split"]].get(x, 0) + 1
    (dest / "d7-summary.json").write_text(json.dumps(dict(rows=len(rows), by_split={s: sum(r["split"] == s for r in rows) for s in ["train", "test"]}, groups=len({r["group"] for r in rows}), label_counts=counts, members=[dict(id=r["id"], url=r["url"], split=r["split"], labels=r["labels"], first_response=r["first_response"]) for r in rows], sha256=hashlib.sha256((root / "d7.jsonl").read_bytes()).hexdigest(), bodies_in_git=False), indent=1))
    manifest = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in dest.glob("*.json") if p.name != "manifest.json"}
    (dest / "manifest.json").write_text(json.dumps(dict(files=manifest, source_bodies=False, weights=False, labels="agent-provisional for evaluation only; no independent human review"), indent=1))
    print("published", len(manifest))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    publish(p.parse_args().root)
