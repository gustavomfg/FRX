"""Seals the v8 decision code by hash. Test cohorts refuse to run against changed code."""
import argparse
import hashlib
import json
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
CACHE = Path.home() / ".cache/frx-ai-v8-20261030"
FILES = ["detector.py", "joint.py", "investigator.py", "trajectory.py", "logs.py", "score.py", "compare.py", "reserve.py", "evaluate.py"]
PRIOR = ["signals.py", "knowledge.py", "investigate.py", "procedures.py"]


def hashes():
    out = {f: hashlib.sha256((HERE / f).read_bytes()).hexdigest() for f in FILES}
    out.update({f"v7/{f}": hashlib.sha256((HERE.parent / "v7" / f).read_bytes()).hexdigest() for f in PRIOR})
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
