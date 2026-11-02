"""Copies metrics, hashes and labels to snapshot/, without comment bodies or quotes."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CACHE = Path.home() / ".cache/frx-ai-v8-20261030"
DEST = HERE / "snapshot"
COHORTS = ["o8-dev", "o8-test", "t8-dev", "t8-test", "l8"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def main():
    DEST.mkdir(exist_ok=True)
    first = json.loads((CACHE / "first-reading.json").read_text())
    for row in first["outcomes"]["details"]:
        row.pop("quotes", None)
    (DEST / "first-reading.json").write_text(json.dumps(first, indent=1))
    (DEST / "posthoc.json").write_text((CACHE / "posthoc.json").read_text())
    (DEST / "frozen.json").write_text((CACHE / "frozen.json").read_text())
    (DEST / "label-lock.json").write_text((CACHE / "label-lock.json").read_text())
    manifest = {}
    for name in COHORTS:
        rows = read(CACHE / f"{name}-reserved.jsonl")
        manifest[name] = dict(n=len(rows), sha256=sha(CACHE / f"{name}-reserved.jsonl"), urls=[r["url"] for r in rows])
    (DEST / "cohorts.json").write_text(json.dumps(manifest, indent=1))
    labels = {}
    for name in ("o8-dev", "o8-test"):
        labels[name] = [{k: g[k] for k in ("id", "annotator", "review_level", "role", "overall", "relation")} for g in read(CACHE / f"{name}-gold.jsonl")]
    for name in ("t8-dev", "t8-test"):
        labels[name] = [dict(id=g["id"], annotator=g["annotator"], review_level=g["review_level"], decisions=g["decisions"], step_kinds=[s["kind"] for s in g["steps"]]) for g in read(CACHE / f"{name}-gold.jsonl")]
    labels["l8"] = [{k: g.get(k) for k in ("id", "annotator", "review_level", "signals", "module")} for g in read(CACHE / "l8-gold.jsonl")]
    (DEST / "labels-without-text.json").write_text(json.dumps(labels, indent=1))
    print("snapshot written")


if __name__ == "__main__":
    main()
