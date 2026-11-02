"""Copies metrics, hashes and labels to snapshot/, without comment text or quotes."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CACHE = Path.home() / ".cache/frx-ai-v9-20261031"
DEST = HERE / "snapshot"


def strip(row):
    for f in row.get("findings", []):
        f.pop("quote", None)
    return row


def main():
    DEST.mkdir(exist_ok=True)
    first = json.loads((CACHE / "first-reading.json").read_text())
    first["rows"] = [strip(r) for r in first["rows"]]
    (DEST / "first-reading.json").write_text(json.dumps(first, indent=0))
    for name in ("posthoc.json", "replays.json", "frozen.json", "frozen-before-harness-fix.json", "label-lock.json", "judgments.json", "live-results.json", "live-incidents.json"):
        file = CACHE / name
        if file.exists():
            (DEST / name).write_text(file.read_text())
    multi = json.loads((CACHE / "multiturn.json").read_text())
    (DEST / "multiturn.json").write_text(json.dumps(multi, indent=0))
    for name in ("dev", "test"):
        cases = [json.loads(x) for x in (CACHE / f"{name}-reserved.jsonl").read_text().splitlines()]
        (DEST / f"{name}-cases.json").write_text(json.dumps([dict(id=c["id"], stratum=c["stratum"], type=c["type"], cutoff=c["cutoff"], url=c["url"]) for c in cases], indent=0))
        (DEST / f"{name}-gold.json").write_text(json.dumps([json.loads(x) for x in (CACHE / f"{name}-gold.jsonl").read_text().splitlines()], indent=0))
    print("snapshot written")


if __name__ == "__main__":
    main()
