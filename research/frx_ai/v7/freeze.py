"""Hash the frozen generator and decision before P7 bodies are opened; verify later."""
import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
FILES = [HERE / "procedures.py", HERE / "l1_adapter.py", HERE / "waterfall.py", HERE.parent / "v6/grounding.py", HERE.parent / "v6/evaluate.py", HERE.parent / "v6/pairs.py", HERE.parent / "v5/events.py", HERE.parent / "v5/relations.py"]


def hashes():
    return {str(p.relative_to(HERE.parent)): hashlib.sha256(p.read_bytes()).hexdigest() for p in FILES}


INVESTIGATOR = [HERE / "investigate.py", HERE / "knowledge.py", HERE / "signals.py", HERE / "retrieve.py", HERE / "c7.py"]


def investigator_hashes():
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in INVESTIGATOR}


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--verify", action="store_true")
    a = p.parse_args()
    dest = a.root / "frozen.json"
    if a.verify:
        stored = json.loads(dest.read_text())
        changed = [k for k, v in hashes().items() if stored["files"].get(k) != v]
        print(dict(ok=not changed, changed=changed))
        raise SystemExit(bool(changed))
    if dest.exists():
        raise SystemExit("preserve first freeze")
    selection = dict(decision="v7-structural", threshold=3.4, selected_on=["P6-dev", "P6-known"], rejected=dict(v6_L1_over_v7_candidates="16 false successes on P6-known, 4 on P6-dev"), selection_is_not_test=True)
    dest.write_text(json.dumps(dict(frozen_at=datetime.now(timezone.utc).isoformat(), files=hashes(), selection=selection, p7_bodies_opened=False), indent=1))
    print(json.dumps(selection, indent=1))
