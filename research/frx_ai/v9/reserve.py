"""Seeded case draws by stratum. Reports counts only; a test draw needs frozen code."""
import argparse
import json
import random
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "benchmark"))
import net  # noqa: E402
import pilot  # noqa: E402
import reading as rd  # noqa: E402

CACHE = net.CACHE
SEED = "frx-v9-20261031"
MAINTAINER = ("MEMBER", "OWNER", "COLLABORATOR")
STRATA = dict(
    library=r"(?i)\bnot found\b|\bmissing\b.{0,40}\b(?:dll|library|lib)\b|\.dll\b|cannot open shared object",
    graphics=r"(?i)\bartifact|flicker|glitch|corrupt|black screen|graphic",
    regression=r"(?i)\bregress|used to work|worked (?:on|in|with|before)|stopped working",
    driver=r"(?i)\b(?:nvidia|mesa|amdgpu|radv)\b.{0,60}\b(?:driver|crash|hang|fault)",
    audio=r"(?i)\b(?:audio|sound|crackl|no sound)\b",
    crash=r"(?i)\bcrash|segfault|unhandled exception",
    performance=r"(?i)\b(?:fps|stutter\w*|performance|lag)\b",
    any=r".",
)
PLAN = dict(dev=1, test=3)
COUNTS = dict(dev=None, test=None)
SIZE = dict(dev=6, test=24)
NOISE = re.compile(r"(?is)<!--.*?-->|(?m:^\s*[-*]\s*\[[ xX*]\][^\n]*$)")
SYMPTOM_LINE = re.compile(r"(?is)\**#*\s*Symptoms?\s*\**:?\s*(.+?)(?:\n\s*#{1,4}\s*\w|\Z)")
TEMPLATE_LINE = re.compile(r"(?im)^\s*(?:#+\s*(?:Compatibility Report|System Information|I confirm|Reproduction)[^\n]*|[-*\s]*(?:Name of the game|Steam AppID|GPU|Video driver|Driver|Kernel|Link to full system|Proton version|Distro|KDE|CPU|RAM)[^\n]*)$")
FIELD = lambda label: re.compile(rf"(?im)^[-*\s]*{label}[^:\n]*:\s*(.+)$")


def excluded():
    known = pilot.known_threads()
    known |= {r["thread"] for r in pilot.read(pilot.ROOT / "sampling/strata.jsonl")}
    v8 = Path.home() / ".cache/frx-ai-v8-20261030"
    for file in v8.glob("*-reserved.jsonl"):
        known |= {r["thread"] for r in pilot.read(file)}
    for file in CACHE.glob("*-reserved.jsonl"):
        known |= {r["thread"] for r in pilot.read(file)}
    return known


def input_of(issue, text):
    """What the person said at that moment. The rest of the thread stays hidden."""
    title = issue.get("title") or ""
    m = re.match(r"^(.*?)\s*\((\d{2,})\)\s*$", title)
    body = NOISE.sub(" ", text or "")
    symptom = SYMPTOM_LINE.search(body)
    summary = (symptom.group(1) if symptom and len(symptom.group(1).strip()) > 20 else TEMPLATE_LINE.sub("", body)).strip()
    get = lambda label: (lambda x: x.group(1).strip()[:120] if x else None)(FIELD(label).search(body))
    return dict(game=m.group(1) if m else None, app_id=m.group(2) if m else None, title=title, gpu=get("GPU"), driver=get("Video driver") or get("Driver"), proton=get("Proton version"), symptom=rd.logs.redact(summary)[:700], log=None)


def candidates(by_thread, issues, known):
    for n, rows in by_thread.items():
        issue = issues[n]
        if n in known or len(rows) < 3 or issue.get("comments", 0) > len(rows):
            continue
        if issue["created_at"] >= "2023-11-01" and issue.get("body") and len(input_of(issue, issue["body"])["symptom"]) >= 80:
            yield dict(thread=n, type="new-thread", cutoff=issue["created_at"], text=issue["body"], later=rows, exclude=[n])
            continue
        if issue["created_at"] >= "2023-11-01":
            continue
        for i, c in enumerate(rows[:-2]):
            if c["author_association"] not in MAINTAINER and 80 <= len(c.get("body") or "") <= 2500 and any(re.search(STRATA[k], c["body"]) for k in ("crash", "graphics", "performance", "audio")) and not c["body"].lstrip().startswith(">"):
                yield dict(thread=n, type="mid-thread", cutoff=c["created_at"], text=c["body"], later=rows[i + 1:], exclude=[])
                break


def draw(name):
    dest = CACHE / f"{name}-reserved.jsonl"
    if dest.exists():
        raise SystemExit(f"{name} already drawn")
    if name == "test" and not (CACHE / "frozen.json").exists():
        raise SystemExit("the test cohort needs frozen code first")
    rng = random.Random(f"{SEED}/{name}")
    issues = {r["number"]: r for r in pilot.read(pilot.CORPUS / "issues.jsonl")}
    by_thread = {}
    for c in pilot.read(pilot.CORPUS / "comments.jsonl"):
        t = pilot.thread_of(c)
        if t in issues:
            by_thread.setdefault(t, []).append(c)
    for rows in by_thread.values():
        rows.sort(key=lambda c: c["created_at"])
    known = excluded()
    pool = sorted(candidates(by_thread, issues, known), key=lambda x: x["thread"])
    chosen, used = [], set()
    per = PLAN[name]
    for stratum, rx in STRATA.items():
        options = [c for c in pool if c["thread"] not in used and re.search(rx, c["text"])]
        for c in rng.sample(options, min(per, len(options))):
            if len([x for x in chosen if x["stratum"] == stratum]) == per:
                break
            used.add(c["thread"])
            chosen.append(dict(id=f"{name}-{c['thread']}", thread=c["thread"], stratum=stratum, type=c["type"], cutoff=c["cutoff"], exclude=c["exclude"], url=issues[c["thread"]]["html_url"], input=input_of(issues[c["thread"]], c["text"]),
                               hidden=[dict(role="maintainer" if x["author_association"] in MAINTAINER else "other", at=x["created_at"], url=x["html_url"], text=x["body"]) for x in c["later"]]))
    chosen = chosen[:SIZE[name]]
    pilot.write_rows(dest, chosen)
    print(json.dumps(dict(cohort=name, n=len(chosen), sha256=pilot.sha(dest), excluded=len(known), pool=len(pool), strata=[c["stratum"] for c in chosen], types=[c["type"] for c in chosen])))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("cohort", choices=sorted(PLAN))
    draw(p.parse_args().cohort)
