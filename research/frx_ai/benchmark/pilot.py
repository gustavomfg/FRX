#!/usr/bin/env python3
"""Pilot of the independent benchmark: blind sampling, annotation, agreement, separate gold.

Agents may run this code but never annotate or adjudicate. Sampling reports counts only,
so whoever runs it does not read the cases. No model is imported here.
"""
import argparse
import hashlib
import json
import os
import random
import re
import sys
from collections import Counter
from pathlib import Path

HOME = Path.home()
ROOT = HOME / ".cache/frx-ai-benchmark-pilot-20261030"
CORPUS = HOME / ".cache/frx-ai-rnd-v7-20261030/corpus"
SEED = "frx-benchmark-pilot-20261030"
SIZE = dict(outcome_vocabulary=20, several_actions=10, unscreened=10)

CACHES = ["frx-ai-rnd-20261029", "frx-ai-rnd-v2-20261029", "frx-ai-rnd-v3-20261030", "frx-ai-rnd-v4-20261030",
          "frx-ai-rnd-v5-20261030", "frx-ai-rnd-v6-20261030", "frx-ai-rnd-v7-20261030", "frx-ai-v6-known-replay-20261030"]
LABELLED = ("*gold*.jsonl", "*audited*.jsonl", "*known*.jsonl", "train*.jsonl", "dev*.jsonl", "final*.jsonl", "calibration*.jsonl",
            "retrieval-*.jsonl", "weak-events.jsonl", "*reserved*.jsonl", "d7.jsonl", "c7-cases.jsonl", "final-annotations.json")

# Wide on purpose: this screen only sets sampling strata and decides nothing about a case.
VOCABULARY = re.compile(r"(?i)\b(?:work\w*|fix\w*|solv\w*|resolv\w*|help\w*|crash\w*|still|same|worse|better|broke\w*|run\w*|launch\w*)\b")
ACTIONS = re.compile(r"(?i)\b(?:tried|installed|removed|deleted|set|added|switched|updated|disabled|enabled|changed|used|using|PROTON_\w+|WINE\w+|DXVK_\w+|winetricks|protontricks)\b")
BOT = re.compile(r"(?i)\[bot\]$")

CLAIMS = ("resolved", "improved", "not_resolved", "worsened", "mixed")
OVERALL = CLAIMS + ("no_claim", "unclear")
ROLES = ("first_hand", "second_hand", "request_or_question", "advice", "information", "unclear")
RELATIONS = ("single", "joint", "alternatives", "sequence", "unclear", "not_applicable")
GUARD_ENV = ("CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "CODEX_SANDBOX", "CODEX_THREAD_ID")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def write_rows(path, rows):
    Path(path).write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))


def known_threads():
    """Every issue that any v1-v7 labelled or reserved file refers to."""
    seen = set()
    for cache in CACHES:
        base = HOME / ".cache" / cache
        for pattern in LABELLED:
            for file in base.glob(pattern):
                seen |= {int(x) for x in re.findall(r"Proton/issues/(\d+)", file.read_text(errors="ignore"))}
        reservation = base / "reservation.json"
        if reservation.exists():
            seen |= {int(x) for x in re.findall(r'"number":\s*(\d+)', reservation.read_text())}
    return seen


def eligible(comment):
    body = comment.get("body") or ""
    user = comment.get("user") or {}
    if comment.get("author_association") in ("MEMBER", "OWNER", "COLLABORATOR") or user.get("type") == "Bot" or BOT.search(user.get("login") or ""):
        return False
    return 60 <= len(body) <= 1500


def thread_of(comment):
    """Issue number, or None for a pull request comment."""
    m = re.search(r"/issues/(\d+)", comment["html_url"])
    return int(m.group(1)) if m else None


def stratum(body):
    if len(set(x.lower() for x in ACTIONS.findall(body))) >= 2:
        return "several_actions"
    return "outcome_vocabulary" if VOCABULARY.search(body) else "other"


def sample(root=ROOT, corpus=CORPUS):
    root = Path(root)
    packet = root / "packet"
    if (packet / "cases.jsonl").exists():
        raise SystemExit("the pilot sample already exists; a new draw needs a new directory")
    packet.mkdir(parents=True, exist_ok=True)
    known = known_threads()
    titles = {r["number"]: r["title"] for r in read(Path(corpus) / "issues.jsonl")}
    pools = {k: [] for k in ("outcome_vocabulary", "several_actions", "other")}
    for c in read(Path(corpus) / "comments.jsonl"):
        if not eligible(c):
            continue
        t = thread_of(c)
        if t is not None and t not in known and t in titles:
            pools[stratum(c["body"])].append(c)
    rng = random.Random(SEED)
    chosen, used = [], set()

    def take(pool, n, name):
        taken = 0
        for c in rng.sample(sorted(pool, key=lambda x: x["id"]), len(pool)):
            if taken == n:
                break
            t = thread_of(c)
            if t in used:
                continue
            used.add(t)
            chosen.append((c, name))
            taken += 1
        if taken < n:
            raise SystemExit(f"stratum {name} holds only {taken} of {n}")

    take(pools["outcome_vocabulary"], SIZE["outcome_vocabulary"], "outcome_vocabulary")
    take(pools["several_actions"], SIZE["several_actions"], "several_actions")
    take(pools["outcome_vocabulary"] + pools["several_actions"] + pools["other"], SIZE["unscreened"], "unscreened")
    rng.shuffle(chosen)
    cases, private = [], []
    for i, (c, name) in enumerate(chosen):
        cid = f"pilot-{i:02d}"
        t = thread_of(c)
        cases.append(dict(id=cid, title=titles[t], url=c["html_url"], text=c["body"]))
        private.append(dict(id=cid, stratum=name, thread=t, created_at=c["created_at"], source_sha256=hashlib.sha256(c["body"].encode()).hexdigest()))
    write_rows(packet / "cases.jsonl", cases)
    manifest = dict(seed=SEED, cases_sha256=sha(packet / "cases.jsonl"), n=len(cases), strata=dict(Counter(p["stratum"] for p in private)),
                    excluded_known_threads=len(known), pool=dict((k, len(v)) for k, v in pools.items()), read_by_agent=False,
                    source="ValveSoftware/Proton public comments in the local v7 snapshot")
    (root / "sampling").mkdir(exist_ok=True)
    write_rows(root / "sampling/strata.jsonl", private)
    (root / "sampling/manifest.json").write_text(json.dumps(manifest, indent=1))
    print(json.dumps(manifest, indent=1))


# Agreement ------------------------------------------------------------------


def krippendorff_nominal(units):
    """Units are lists of labels, one per annotator; None is a missing label."""
    pairs, values = Counter(), Counter()
    for unit in units:
        labels = [x for x in unit if x is not None]
        m = len(labels)
        if m < 2:
            continue
        for i, a in enumerate(labels):
            values[a] += 1
            for j, b in enumerate(labels):
                if i != j:
                    pairs[(a, b)] += 1 / (m - 1)
    n = sum(values.values())
    if n < 2:
        return None
    observed = sum(v for (a, b), v in pairs.items() if a != b)
    expected = sum(values[a] * values[b] for a in values for b in values if a != b) / (n - 1)
    return 1.0 if expected == 0 else 1 - observed / expected


def cohen_kappa(a, b):
    both = [(x, y) for x, y in zip(a, b) if x is not None and y is not None]
    if not both:
        return None
    n = len(both)
    po = sum(x == y for x, y in both) / n
    ca, cb = Counter(x for x, _ in both), Counter(y for _, y in both)
    pe = sum(ca[k] * cb[k] for k in ca) / (n * n)
    return 1.0 if pe == 1 else (po - pe) / (1 - pe)


def span_f1(a, b):
    """Character overlap of two sets of literal evidence spans, micro over one unit pair."""
    x, y = set(), set()
    for s, e in a:
        x |= set(range(s, e))
    for s, e in b:
        y |= set(range(s, e))
    if not x and not y:
        return None
    return 2 * len(x & y) / (len(x) + len(y))


def bootstrap_alpha(units, n=2000):
    rng = random.Random(SEED)
    values = []
    for _ in range(n):
        v = krippendorff_nominal([units[rng.randrange(len(units))] for _ in units])
        if v is not None:
            values.append(v)
    values.sort()
    return [values[int(.025 * len(values))], values[int(.975 * len(values))]] if values else None


def verdict(alpha):
    if alpha is None:
        return "not computable"
    return "acceptable" if alpha >= .8 else "tentative" if alpha >= .667 else "below threshold; revise the guide or the field"


def validate(row, cases):
    case = cases.get(row.get("id"))
    if case is None:
        return f"unknown case {row.get('id')}"
    if row.get("role") not in ROLES or row.get("overall") not in OVERALL or row.get("relation") not in RELATIONS:
        return f"{row['id']}: value outside the closed lists"
    if row.get("assistance") != "none":
        return f"{row['id']}: annotation does not declare the absence of model or LLM assistance"
    if row["overall"] in CLAIMS and not row.get("evidence"):
        return f"{row['id']}: a claim needs literal evidence"
    for quote in row.get("evidence", []) + row.get("actions", []):
        if quote not in case["text"]:
            return f"{row['id']}: quote is not literal: {quote[:40]!r}"
    return None


def load_annotation(root, name, cases):
    file = Path(root) / "annotations" / f"{name}.jsonl"
    if not file.exists():
        return None
    rows = read(file)
    errors = [e for e in (validate(r, cases) for r in rows) if e]
    if errors:
        raise SystemExit("invalid annotation file: " + "; ".join(errors[:5]))
    return {r["id"]: r for r in rows}


def spans(text, quotes):
    out = []
    for q in quotes:
        i = text.find(q)
        if i >= 0:
            out.append((i, i + len(q)))
    return out


def registered(root):
    file = Path(root) / "annotators.json"
    return json.loads(file.read_text()) if file.exists() else {}


def review_level(people):
    """H2 needs two different people declared independent of the system; anything less says so."""
    independent = [p for p in people if p.get("independent_of_frx") is True and p.get("kind") == "human"]
    if len({p["id"] for p in independent}) >= 2:
        return "H2"
    return "H1" if any(p.get("kind") == "human" for p in people) else "H0"


def agree(root=ROOT, a="A", b="B"):
    root = Path(root)
    cases = {r["id"]: r for r in read(root / "packet/cases.jsonl")}
    first, second = load_annotation(root, a, cases), load_annotation(root, b, cases)
    if first is None or second is None:
        raise SystemExit("two annotation files are required")
    reg = registered(root)
    people = [reg.get(a, {}), reg.get(b, {})]
    ids = [i for i in cases if i in first and i in second]
    result = dict(n=len(ids), cases=len(cases), review_level=review_level([p for p in people if p]), fields={})
    for field in ("role", "overall", "relation"):
        units = [[first[i][field], second[i][field]] for i in ids]
        alpha = krippendorff_nominal(units)
        result["fields"][field] = dict(alpha=alpha, ci95=bootstrap_alpha(units) if units else None, kappa=cohen_kappa(*zip(*units)) if units else None,
                                       raw_agreement=sum(x == y for x, y in units) / len(units) if units else None, verdict=verdict(alpha))
    has = [[first[i]["overall"] in CLAIMS, second[i]["overall"] in CLAIMS] for i in ids]
    alpha = krippendorff_nominal(has)
    result["fields"]["has_claim"] = dict(alpha=alpha, ci95=bootstrap_alpha(has) if has else None, verdict=verdict(alpha))
    f1 = [span_f1(spans(cases[i]["text"], first[i].get("evidence", [])), spans(cases[i]["text"], second[i].get("evidence", []))) for i in ids]
    f1 = [x for x in f1 if x is not None]
    result["evidence_overlap_f1"] = sum(f1) / len(f1) if f1 else None
    result["confusion_overall"] = {f"{x}|{y}": n for (x, y), n in Counter((first[i]["overall"], second[i]["overall"]) for i in ids).items()}
    result["disagreements"] = [i for i in ids if first[i]["overall"] != second[i]["overall"] or first[i]["relation"] != second[i]["relation"]]
    result["claim"] = "independent" if result["review_level"] == "H2" else "not independent: " + result["review_level"]
    (root / "agreement.json").write_text(json.dumps(result, indent=1))
    print(json.dumps({k: v for k, v in result.items() if k != "confusion_overall"}, indent=1))
    return result


def adjudicate(root=ROOT, a="A", b="B", decisions=None):
    """Agreed rows pass through; each disagreement needs a human adjudicator's decision row. Gold lives apart."""
    root = Path(root)
    cases = {r["id"]: r for r in read(root / "packet/cases.jsonl")}
    first, second = load_annotation(root, a, cases), load_annotation(root, b, cases)
    decided = {r["id"]: r for r in read(decisions)} if decisions else {}
    gold, missing = [], []
    for i in cases:
        x, y = first.get(i), second.get(i)
        if x and y and x["overall"] == y["overall"] and x["relation"] == y["relation"] and x["role"] == y["role"]:
            gold.append(dict(x, source="agreement", annotators=[a, b]))
        elif i in decided:
            problem = validate(decided[i], cases)
            if problem:
                raise SystemExit(problem)
            gold.append(dict(decided[i], source="adjudication", annotators=[a, b]))
        else:
            missing.append(i)
    if missing:
        raise SystemExit(f"{len(missing)} cases need adjudication: {missing}")
    out = root / "gold"
    out.mkdir(mode=0o700, exist_ok=True)
    dest = out / "pilot-gold.jsonl"
    if dest.exists():
        raise SystemExit("gold already exists")
    write_rows(dest, gold)
    os.chmod(dest, 0o600)
    (root / "gold-lock.json").write_text(json.dumps(dict(sha256=sha(dest), n=len(gold), review_level=review_level([registered(root).get(a, {}), registered(root).get(b, {})])), indent=1))
    print(f"gold written with {len(gold)} cases; lock hash recorded")


def lock_predictions(root, system, predictions):
    """A system's predictions must be sealed by hash before any score exists."""
    root = Path(root)
    dest = root / "prediction-locks.json"
    locks = json.loads(dest.read_text()) if dest.exists() else {}
    if system in locks:
        raise SystemExit("this system is already locked")
    if (root / "gold-lock.json").exists():
        raise SystemExit("gold already exists: a lock after gold is not blind")
    locks[system] = dict(sha256=sha(predictions), path=str(predictions))
    dest.write_text(json.dumps(locks, indent=1))


def annotate_cli(root, name):
    """Terminal annotation. Shows only the text: no prediction, model or other annotator."""
    for key in GUARD_ENV:
        if os.environ.get(key):
            raise SystemExit(f"annotation refused inside an agent session ({key}). Gold must come from a person.")
    root = Path(root)
    cases = read(root / "packet/cases.jsonl")
    (root / "annotations").mkdir(exist_ok=True)
    file = root / "annotations" / f"{name}.jsonl"
    done = {r["id"] for r in read(file)} if file.exists() else set()

    def closed(prompt, options):
        while True:
            answer = input(f"{prompt} {list(options)}: ").strip()
            if answer in options:
                return answer

    def quotes(prompt, text):
        out = []
        while True:
            q = input(f"{prompt} (trecho literal; Enter encerra): ")
            if not q:
                return out
            if q in text:
                out.append(q)
            else:
                print("Trecho não encontrado literalmente no texto.")

    for case in cases:
        if case["id"] in done:
            continue
        print("\n" + "=" * 72 + f"\n{case['id']} — {case['title']}\n{case['url']}\n\n{case['text']}\n")
        row = dict(id=case["id"], annotator=name, role=closed("Papel", ROLES), overall=closed("Resultado geral", OVERALL))
        row["evidence"] = quotes("Evidência", case["text"]) if row["overall"] in CLAIMS else []
        row["actions"] = quotes("Ação", case["text"])
        row["relation"] = closed("Relação entre ações", RELATIONS)
        row["note"] = input("Nota (opcional): ").strip()
        row["assistance"] = "none" if closed("Anotou sem assistência de modelo ou LLM?", ("sim", "nao")) == "sim" else "declared"
        with file.open("a") as f:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")


def status(root=ROOT):
    root = Path(root)
    manifest = json.loads((root / "sampling/manifest.json").read_text())
    files = sorted(p.stem for p in (root / "annotations").glob("*.jsonl")) if (root / "annotations").exists() else []
    print(json.dumps(dict(cases=manifest["n"], cases_unchanged=sha(root / "packet/cases.jsonl") == manifest["cases_sha256"], strata=manifest["strata"],
                          annotators=files, review_level=review_level(list(registered(root).values())), gold_exists=(root / "gold/pilot-gold.jsonl").exists()), indent=1))


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("command", choices=["sample", "status", "annotate", "agree", "adjudicate"])
    p.add_argument("--root", type=Path, default=ROOT)
    p.add_argument("--annotator", default="A")
    p.add_argument("--decisions", type=Path)
    a = p.parse_args()
    if a.command == "sample":
        sample(a.root)
    elif a.command == "status":
        status(a.root)
    elif a.command == "annotate":
        annotate_cli(a.root, a.annotator)
    elif a.command == "agree":
        agree(a.root)
    else:
        adjudicate(a.root, decisions=a.decisions)
    sys.exit(0)
