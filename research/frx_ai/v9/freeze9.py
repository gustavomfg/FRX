"""Seals the v9 decision code by hash before a test cohort is drawn."""
import argparse
import hashlib
import json
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
CACHE = Path.home() / ".cache/frx-ai-v9-20261031"
FILES = ["net.py", "sources.py", "reading.py", "engine.py", "evaluate.py", "reserve.py"]
PRIOR = ["v8/detector.py", "v8/joint.py", "v8/investigator.py", "v8/logs.py", "v7/knowledge.py", "v7/signals.py", "v7/investigate.py"]


def hashes():
    out = {f: hashlib.sha256((HERE / f).read_bytes()).hexdigest() for f in FILES}
    out.update({f: hashlib.sha256((HERE.parent / f).read_bytes()).hexdigest() for f in PRIOR})
    return out


def changed():
    state = json.loads((CACHE / "frozen.json").read_text())
    return [k for k, v in hashes().items() if state["files"].get(k) != v]


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--verify", action="store_true")
    a = p.parse_args()
    if a.verify:
        c = changed()
        print(json.dumps(dict(ok=not c, changed=c)))
    else:
        dest = CACHE / "frozen.json"
        if dest.exists():
            raise SystemExit("already frozen")
        dest.write_text(json.dumps(dict(frozen_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), files=hashes()), indent=1))
        print("frozen", len(hashes()))
