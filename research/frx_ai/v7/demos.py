"""D7: what maintainers actually asked next, from public Proton issue threads.

The state is everything visible before the maintainer's comment; the label comes
from deterministic functions over the maintainer's own words. Agent-written
annotations never enter this dataset.
"""
import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

SEED = "frx-v7-d7-20261030"
MAINTAINER = ("MEMBER", "COLLABORATOR", "OWNER")
TITLE = re.compile(r"\((\d{3,10})\)\s*$")

# Closed action space, fixed before reading D7 test data.
CHECKS = [
    "collect-proton-log", "collect-system-info", "collect-terminal-output", "test-other-proton",
    "reset-prefix", "review-launch-options", "test-runtime-option", "check-driver-stack",
    "verify-game-files", "isolate-scope",
]
LABELERS = {
    "collect-proton-log": r"PROTON_LOG|steam-\$?\w*\.log|proton log",
    "collect-system-info": r"Steam Runtime Diagnostics|system information|Help\s*(?:->|>)\s*(?:System|Steam Runtime)",
    "collect-terminal-output": r"from a terminal|terminal (?:output|spew)|console[-_]linux|console_log|content_log|coredump|core dump|backtrace|journalctl|dmesg|\bgdb\b|strace|apitrace|vulkaninfo",
    "test-other-proton": r"\b(?:try|test|retest|check|switch\w*|force|use|see if|does)\b[^.\n]{0,60}\b(?:proton\s*(?:experimental|hotfix|\d)|bleeding[- ]edge|experimental|hotfix)\b",
    "reset-prefix": r"\b(?:remov\w*|delet\w*|renam\w*|clear\w*|fresh|new|nuke\w*|recreat\w*|rebuild\w*)\b[^.\n]{0,50}\b(?:wineprefix|prefix|compatdata)\b",
    "review-launch-options": r"\blaunch options?\b[^.\n]{0,60}\b(?:remov\w*|without|clear\w*|bad|disable\w*|what|any)\b|\b(?:remov\w*|without|clear\w*)\b[^.\n]{0,40}\blaunch options?\b|\bmods?\b[^.\n]{0,40}\b(?:remov\w*|without|disable\w*)\b",
    "test-runtime-option": r"\b(?:PROTON_(?!LOG)[A-Z0-9_]+|WINE[A-Z]*(?:_[A-Z]+)+|DXVK_[A-Z_]+|VKD3D_[A-Z_]+|RADV_[A-Z_]+|WINEDLLOVERRIDES|gamescope|winetricks|protontricks)\b",
    "check-driver-stack": r"\b(?:driver version|update\w* (?:your |the )?(?:driver|mesa|nvidia)|mesa(?:-git| version)?|nvidia driver|graphics driver|kernel version|newer driver|which driver)\b",
    "verify-game-files": r"\bverify\w*\b[^.\n]{0,40}\b(?:files|integrity)\b|\breinstall\w*\b",
    "isolate-scope": r"\b(?:on windows|other games?|another game|different (?:gpu|machine|system|distro)|same (?:hardware|system)|still happen\w*|reproduce\w* (?:on|with|in)|does this (?:happen|occur)|fresh install|steam deck)\b",
}
COMPILED = {k: re.compile("(?i)" + v) if k != "test-runtime-option" else re.compile(v) for k, v in LABELERS.items()}
ASKING = re.compile(r"(?i)\b(?:please|could you|can you|would you|try|test|does|do you|is there|have you|what|which|let us know|if you)\b|\?")


def label(text):
    body = re.sub(r"(?m)^\s*>.*$", "", text or "")
    # Pasted logs and code are evidence in the reply, not what was asked.
    body = re.sub(r"```[\s\S]*?(?:```|\Z)", " ", body)
    if not ASKING.search(body):
        return []
    return [k for k, rx in COMPILED.items() if rx.search(body)]


HAS_LOG = re.compile(r"(?i)steam-\d+\.log|\berr:\w+:|\bfixme:\w+:|\bwine: |\bwarn:\w+:")
HAS_INFO = re.compile(r"(?i)gist\.github|Processor Information|Operating System Version|Driver Version|Mesa \d|NVIDIA \d{3}|\bGPU: *\S|Kernel(?: version)?: *\d")
HAS_OPTION = re.compile(r"PROTON_[A-Z0-9_]+|WINEDLLOVERRIDES|%command%|gamescope|launch options?", re.I)
HAS_VERSION = re.compile(r"(?i)proton[- ]?(?:experimental|ge|\d)|GE-Proton|bleeding")
HAS_PREFIX = re.compile(r"(?i)\b(?:wine)?prefix\b|compatdata")
HAS_VERIFIED = re.compile(r"(?i)\bverif\w+\b[^.\n]{0,30}\b(?:files|integrity)\b|\breinstall\w*\b")
HAS_WINDOWS = re.compile(r"(?i)\bon windows\b|other games?\b|another game\b")
HAS_TERMINAL = re.compile(r"(?i)backtrace|coredump|terminal|journalctl|dmesg")


def reported(text):
    """What the person wrote: the issue template's own hints are not their evidence."""
    text = re.sub(r"<!--[\s\S]*?-->", " ", text or "")
    return re.sub(r"(?im)^.*(?:Issue transferred from|Name of the game with compatibility issues|Steam AppID of the game|## System Information|## I confirm).*$", " ", text)


def state_flags(text):
    return dict(has_log=bool(HAS_LOG.search(text)), has_info=bool(HAS_INFO.search(text)), has_option=bool(HAS_OPTION.search(text)), has_version=bool(HAS_VERSION.search(text)), has_prefix=bool(HAS_PREFIX.search(text)), has_verified=bool(HAS_VERIFIED.search(text)), has_scope=bool(HAS_WINDOWS.search(text)), has_terminal=bool(HAS_TERMINAL.search(text)))


def split_of(group):
    # Reserved before reading any D7 test state; the AppID is the leakage boundary.
    return "test" if int(hashlib.sha256((SEED + group).encode()).hexdigest(), 16) % 5 == 0 else "train"


def build(root):
    meta = json.loads((root / "reservation.json").read_text())
    blocked = set(meta["reserved_app_ids"]) | set(meta.get("j7_app_ids", []))
    issues = {}
    for line in (root / "corpus/issues.jsonl").read_text().splitlines():
        i = json.loads(line)
        if not i.get("pull_request"):
            issues[i["number"]] = i
    threads = defaultdict(list)
    for line in (root / "corpus/comments.jsonl").read_text().splitlines():
        c = json.loads(line)
        threads[int(c["issue_url"].rsplit("/", 1)[1])].append(c)
    rows = []
    skipped = Counter()
    for number, comments in threads.items():
        issue = issues.get(number)
        if issue is None:
            skipped["no-issue"] += 1
            continue
        m = TITLE.search(issue["title"] or "")
        app = m[1] if m else None
        if app in blocked:
            skipped["reserved-app"] += 1
            continue
        group = app or f"issue-{number}"
        comments.sort(key=lambda c: c["created_at"])
        seen_text = []
        since = []
        previous_maintainer = 0
        for c in comments:
            if c["user"]["type"] == "Bot":
                continue
            if "Issue transferred from" in (c.get("body") or "")[:400]:
                # A maintainer moving a report: the words are the reporter's.
                seen_text.append(reported(c.get("body")))
                since.append(seen_text[-1])
                continue
            if c["author_association"] in MAINTAINER:
                labels = label(c.get("body"))
                if labels and seen_text:
                    recent = "\n\n".join(seen_text[-3:])[-4000:]
                    whole = (issue["title"] or "") + "\n\n" + reported(issue.get("body"))[:3000] + "\n\n" + "\n\n".join(seen_text)[-5000:]
                    rows.append(dict(id=f"d7-{c['id']}", group=group, thread=number, app_id=app, split=split_of(group), url=c["html_url"], labels=labels, created_at=c["created_at"], first_response=previous_maintainer == 0, query=(issue["title"] or "") + "\n\n" + recent, whole=whole, flags=state_flags("\n\n".join(since) if previous_maintainer else whole), thread_flags=state_flags(whole)))
                previous_maintainer += 1
                # The maintainer answers whoever spoke since their last reply.
                since = []
            else:
                seen_text.append(reported(c.get("body")))
                since.append(seen_text[-1])
    with (root / "d7.jsonl").open("w") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    train = [r for r in rows if r["split"] == "train"]
    print(dict(rows=len(rows), train=len(train), test=len(rows) - len(train), groups=len({r["group"] for r in rows}), skipped=dict(skipped)))
    print("train label counts", Counter(x for r in train for x in r["labels"]).most_common())
    print("train first-response share", sum(r["first_response"] for r in train) / max(1, len(train)))
    print("train labels per row", Counter(len(r["labels"]) for r in train).most_common())
    return rows


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    build(p.parse_args().root)
