#!/usr/bin/env python3
"""Read-only provenance audit of FRX AI v1-v7. Trains nothing and changes no lab.

Writes the matrix to the path given by --out. It reads lab caches, snapshots, git and the two
named agent session files. It never opens ~/.codex/auth.json or ~/.claude/.credentials.json.
Label bodies and comment text are not copied: only counts, hashes and names reach the output.
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
from collections import Counter
from pathlib import Path

HOME = Path.home()
CACHE = HOME / ".cache"
REPO = Path(__file__).resolve().parents[3]
LAB = REPO / "research/frx_ai"
CODEX_SESSION = Path(os.environ.get("FRX_CODEX_SESSION", "/nonexistent/frx-codex-session"))
CLAUDE_SESSION = Path(os.environ.get("FRX_CLAUDE_SESSION", "/nonexistent/frx-claude-session"))
FORBIDDEN = {"auth.json", ".credentials.json"}

V1, V2, V3, V4 = "frx-ai-rnd-20261029", "frx-ai-rnd-v2-20261029", "frx-ai-rnd-v3-20261030", "frx-ai-rnd-v4-20261030"
V5, V6, V7 = "frx-ai-rnd-v5-20261030", "frx-ai-rnd-v6-20261030", "frx-ai-rnd-v7-20261030"

CODEX, CLAUDE = "gpt-6.1-sol (OpenAI Codex)", "claude-opus-5-5 (Claude Code)"
DIRECT, PROGRAM, MIXED = "direct-agent", "program-by-agent", "direct-plus-program"

# id: lab, file, origin class, agent, how, use.
# "use" separates what can change weights (fit, calibration, selection) from what only measures (eval).
DATASETS = {
    "v1-train": ("v1", (V1, "train.jsonl"), DIRECT, CODEX, "annotate.py, inline labels", "fit"),
    "v1-dev": ("v1", (V1, "dev.jsonl"), DIRECT, CODEX, "annotate.py, inline labels", "selection"),
    "v1-final": ("v1", (V1, "final.jsonl"), DIRECT, CODEX, "annotate.py, inline labels", "eval"),
    "v2-train-round1": ("v2", (V2, "train-round1.jsonl"), DIRECT, CODEX, "annotate_events.py", "fit"),
    "v2-train-round2": ("v2", (V2, "train-round2.jsonl"), DIRECT, CODEX, "annotate_events.py + extend_annotations.py", "fit"),
    "v2-train-events": ("v2", (V2, "train-events.jsonl"), DIRECT, CODEX, "round2 + annotate_additional.py", "fit"),
    "v2-dev-events": ("v2", (V2, "dev-events.jsonl"), DIRECT, CODEX, "annotate_events.py", "selection"),
    "v2-calibration-events": ("v2", (V2, "calibration-events.jsonl"), DIRECT, CODEX, "annotate_events.py", "calibration"),
    "v2-final-events": ("v2", (V2, "final-events.jsonl"), DIRECT, CODEX, "final-labels.tsv typed in the session; build_final_annotations.py", "eval, then v3 selection"),
    "v2-weak-events": ("v2", (V2, "weak-events.jsonl"), PROGRAM, CODEX, "weak.py regex votes", "fit (E14 only)"),
    "v2-train-goals": ("v2", (V2, "train-goals.jsonl"), MIXED, CODEX, "agent spans + prepare_goals.py pairs", "fit"),
    "v2-dev-goals": ("v2", (V2, "dev-goals.jsonl"), MIXED, CODEX, "agent spans + prepare_goals.py pairs", "selection"),
    "v2-calibration-goals": ("v2", (V2, "calibration-goals.jsonl"), MIXED, CODEX, "agent spans + prepare_goals.py pairs", "calibration"),
    "v2-final-goals": ("v2", (V2, "final-goal-events.jsonl"), DIRECT, CODEX, "final-labels.tsv typed in the session", "eval"),
    "v2-retrieval-train": ("v2", (V2, "retrieval-train.jsonl"), DIRECT, CODEX, "annotate_topics.py topics; queries hand-written in retrieval_train.py", "fit"),
    "v2-retrieval-dev": ("v2", (V2, "retrieval-dev.jsonl"), DIRECT, CODEX, "annotate_topics.py", "selection"),
    "v3-final": ("v3", (V3, "final-annotations.json"), DIRECT, CODEX, "inline list in a session heredoc, no saved script", "eval"),
    "v4-train": ("v4", (V4, "train-audited.jsonl"), DIRECT, CODEX, "annotate_known.py", "fit"),
    "v4-dev": ("v4", (V4, "dev-audited.jsonl"), DIRECT, CODEX, "annotate_dev.py", "selection"),
    "v4-final": ("v4", (V4, "final-audited.jsonl"), DIRECT, CODEX, "annotate_final.py", "eval, then v5 selection"),
    "v5-train-known": ("v5", (V5, "train-known.jsonl"), DIRECT, CODEX, "v4-train + v4-dev, reused", "fit"),
    "v5-dev-known": ("v5", (V5, "dev-known.jsonl"), DIRECT, CODEX, "v4-final, reused", "selection"),
    "v5-extra": ("v5", (V5, "extra-audited.jsonl"), DIRECT, CODEX, "annotate_extra.py", "fit"),
    "v5-final": ("v5", (V5, "final-audited.jsonl"), DIRECT, CODEX, "annotate_final.py", "eval"),
    "v6-training-pairs": ("v6", (V6, "training-pairs.jsonl"), MIXED, CODEX, "agent groups + program-built counterclaims and synthetic text (learning.py)", "fit"),
    "v6-development": ("v6", (V6, "development-gold-v2.jsonl"), DIRECT, CODEX, "annotate_development.py + erratum", "selection"),
    "v6-acceptance": ("v6", (V6, "acceptance-gold.jsonl"), DIRECT, CODEX, "annotate_acceptance.py", "eval"),
    "v7-p7": ("v7", (V7, "p7-gold.jsonl"), DIRECT, CLAUDE, "annotate_p7.py", "eval"),
    "v7-j7": ("v7", (V7, "j7-gold.jsonl"), DIRECT, CLAUDE, "annotate_j7.py", "eval"),
    "v7-c7": ("v7", (V7, "c7-gold.json"), DIRECT, CLAUDE, "annotate_c7.py", "eval"),
    "v7-d7": ("v7", (V7, "d7.jsonl"), PROGRAM, CLAUDE, "demos.py regex rules over public maintainer comments", "fit (selector.py only, not default)"),
}

# Generated at training time and never stored as a labelled file.
TEMPLATES = {
    "v1-synthetic": "research/frx_ai/synthetic.py", "v2-synthetic": "research/frx_ai/v2/synthetic.py",
    "v4-synthetic": "research/frx_ai/v4/synthetic.py", "v5-status-synthetic": "research/frx_ai/v5/training_data.py",
    "v5-relation-synthetic": "research/frx_ai/v5/relation_data.py", "v3-structure-simulation": "research/frx_ai/v3/structure.py",
}

M = "MiniLM-L6-v2"
E_ROUNDS = {"E01-frozen-clean": "v2-train-round1", "E02-last2-clean": "v2-train-round1", "E03-last2-synthetic": "v2-train-round1",
            "E04-last2-contrast": "v2-train-round1", "E05-full-clauses": "v2-train-round2", "E06-full-masked": "v2-train-round2",
            "E07-full-contrast005": "v2-train-round2", "E11-expanded-control": "v2-train-events", "E12-curriculum": "v2-train-events",
            "E13-hard-relations": "v2-train-events", "E14-weighted-weak": "v2-train-events", "E15-role-loss": "v2-train-events",
            "E16-edge-pairs": "v2-train-events", "E17-hard-balanced-loss": "v2-train-events"}
NO_TEMPLATE = {"E01-frozen-clean", "E02-last2-clean", "E16-edge-pairs"}

# id: file, lab, fit inputs, selection or calibration inputs, parent weights, status.
WEIGHTS = {}


def weight(name, lab, path, fit, tune=(), base=(M,), parents=(), status="research"):
    WEIGHTS[name] = dict(lab=lab, path=path, fit=list(fit), tune=list(tune), base=list(base), parents=list(parents), status=status)


for n in ("research-only", "augmented-research-only", "focused-research-only", "nli-head-research", "reportability-research"):
    fit = ["v1-train"] + (["v1-synthetic"] if n in ("augmented-research-only", "focused-research-only") else [])
    weight("v1-" + n, "v1", (V1, f"experiments/{n}.joblib"), fit, ["v1-dev"], base=(M, "nli-deberta-v3-xsmall"))
for n, d in E_ROUNDS.items():
    fit = [d] + ([] if n in NO_TEMPLATE else ["v2-synthetic"]) + (["v2-weak-events"] if n == "E14-weighted-weak" else [])
    weight(n, "v2", (V2, f"runs/{n}.pt"), fit, ["v2-dev-events"])
weight("E18-goal-conditioned", "v2", (V2, "runs/E18-goal-conditioned.pt"), ["v2-train-goals", "v2-synthetic"], ["v2-dev-goals"])
for n in ("R01-contrast-1e5", "R02-contrast-3e5"):
    weight(n, "v2", (V2, f"retriever-runs/{n}.pt"), ["v2-retrieval-train"], ["v2-retrieval-dev"])
weight("runtime-relation", "v2", (V2, "runtime-relation/relation-int8.onnx"), [], ["v2-calibration-events"], parents=["E07-full-contrast005"])
weight("runtime-goal", "v2", (V2, "runtime-goal/relation-int8.onnx"), [], ["v2-calibration-goals"], parents=["E18-goal-conditioned"])
weight("runtime-retrieval", "v2", (V2, "runtime-retrieval/retriever-int8.onnx"), [], [], parents=["R02-contrast-3e5"])
weight("runtime-frozen-control", "v2", (V2, "runtime-frozen-control/frozen-int8.onnx"), [], [], status="upstream weights, no FRX labels")
weight("v3-modality", "v3", (V3, "events/modality.joblib"), ["v2-train-events", "v2-dev-events"], ["v2-final-events"], parents=["runtime-retrieval"])
weight("v3-outcome", "v3", (V3, "events/outcome.joblib"), ["v2-train-events", "v2-dev-events"], ["v2-final-events"], base=())
weight("v3-structure", "v3", (V3, "structure/models.json"), ["v3-structure-simulation"], [], base=())
for n, syn in (("S1-legacy-span", 0), ("S2-audited-real", 0), ("S3-contrast-events", 1), ("S4-frozen-body", 1), ("S5-balanced-events", 1), ("S6-balanced-no-contrast", 1)):
    weight(n, "v4", (V4, f"runs/{n}.pt"), ["v4-train"] + (["v4-synthetic"] if syn else []), ["v4-dev"])
weight("runtime-events", "v4", (V4, "runtime-events/events-int8.onnx"), [], [], parents=["S3-contrast-events"])
for n, syn in (("A1-real-paragraph", 0), ("A2-contrast-paragraph", 1), ("A3-contrast-sentence", 1), ("A4-syntax-features", 1), ("A4-coverage-control", 1)):
    weight(n, "v5", (V5, f"atomic-runs/{n}.pt"), ["v5-train-known"] + (["v5-status-synthetic"] if syn else []), ["v5-dev-known"])
weight("A5-audited-data", "v5", (V5, "atomic-runs/A5-audited-data.pt"), ["v5-train-known", "v5-extra", "v5-status-synthetic"], ["v5-dev-known"])
weight("A6-seed-control", "v5", (V5, "refit-check/atomic-runs/A6-seed-control.pt"), ["v5-train-known", "v5-extra", "v5-status-synthetic"], ["v5-dev-known"])
weight("v5-linker-selected", "v5", (V5, "linker-audited-selection.json"), ["v5-train-known", "v5-extra"], ["v5-dev-known"], base=(), status="research; failed eligibility, not used")
weight("L1-structured-real-text", "v6", (V6, "verifiers/L1-structured-real-text.json"), ["v6-training-pairs:real"], ["v6-development"], base=())
weight("L2-structured-contrast", "v6", (V6, "verifiers/L2-structured-contrast.json"), ["v6-training-pairs"], ["v6-development"], base=())
weight("L3-semantic-contrast", "v6", (V6, "verifiers/L3-semantic-contrast.json"), ["v6-training-pairs"], ["v6-development"], base=(), parents=["runtime-retrieval"])
weight("N1-real-text", "v6", (V6, "neural/N1-real-text.pt"), ["v6-training-pairs:real"], ["v6-development"])
for n in ("N2-contrast", "N3-structured-contrast"):
    weight(n, "v6", (V6, f"neural/{n}.pt"), ["v6-training-pairs"], ["v6-development"])
weight("L1-v7-copy", "v7", (V7, "verifiers/L1-structured-real-text.json"), [], [], base=(), parents=["L1-structured-real-text"], status="comparator; no refit")

ANNOTATORS = [(V1, "annotate.py"), (V2, "annotate_events.py"), (V2, "extend_annotations.py"), (V2, "annotate_additional.py"),
              (V2, "annotate_topics.py"), (V2, "build_final_annotations.py"), (V4, "annotate_known.py"), (V4, "annotate_dev.py"),
              (V4, "annotate_final.py"), (V5, "annotate_extra.py"), (V5, "annotate_final.py"), (V6, "annotate_development.py"),
              (V6, "annotate_acceptance.py"), (V7, "annotate_p7.py"), (V7, "annotate_j7.py"), (V7, "annotate_c7.py")]


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def count(path):
    if path.suffix == ".jsonl":
        with open(path, errors="ignore") as f:
            return sum(1 for line in f if line.strip())
    data = json.loads(path.read_text())
    return len(data)


def codex_calls():
    assert CODEX_SESSION.name not in FORBIDDEN
    calls, model, meta, plans, users = [], None, {}, Counter(), []
    with open(CODEX_SESSION, errors="ignore") as f:
        for line in f:
            try:
                o = json.loads(line)
            except ValueError:
                continue
            p, kind = o.get("payload") or {}, o.get("type")
            if kind == "session_meta":
                meta = {k: p.get(k) for k in ("originator", "model_provider", "source", "cwd")}
            elif kind == "turn_context":
                model = p.get("model")
            elif kind == "event_msg":
                limits = (p.get("rate_limits") or (p.get("info") or {}).get("rate_limits") or {}) if isinstance(p, dict) else {}
                if isinstance(limits, dict) and limits.get("plan_type"):
                    plans[limits["plan_type"]] += 1
            elif kind == "response_item":
                if p.get("type") in ("function_call", "custom_tool_call"):
                    calls.append((o["timestamp"], model, str(p.get("arguments") or p.get("input") or "")))
                elif p.get("role") == "user" and o["timestamp"] > "2026-10-29T20":
                    text = " ".join(x.get("text", "") for x in p.get("content", []) if isinstance(x, dict))
                    if not text.startswith("<"):
                        users.append(dict(at=o["timestamp"], chars=len(text)))
    return calls, meta, dict(plans), users


def claude_calls():
    assert CLAUDE_SESSION.name not in FORBIDDEN
    calls, models, entry = [], Counter(), Counter()
    with open(CLAUDE_SESSION, errors="ignore") as f:
        for line in f:
            try:
                o = json.loads(line)
            except ValueError:
                continue
            if o.get("entrypoint"):
                entry[o["entrypoint"]] += 1
            if o.get("type") != "assistant":
                continue
            message = o.get("message") or {}
            models[message.get("model")] += 1
            for c in message.get("content") or []:
                if isinstance(c, dict) and c.get("type") == "tool_use":
                    calls.append((o.get("timestamp") or "", message.get("model"), json.dumps(c.get("input"), ensure_ascii=False)))
    return calls, dict(models), dict(entry)


def authorship(codex, claude):
    """Earliest session call holding the longest literal lines of each annotator."""
    out = []
    for lab_cache, name in ANNOTATORS:
        path = CACHE / lab_cache / name
        lines = sorted((x.strip() for x in path.read_text(errors="ignore").splitlines() if len(x.strip()) > 40), key=len, reverse=True)
        picks = [x[:160] for x in lines[:6]]
        cx = [c for c in codex if any(k in c[2] for k in picks)]
        cl = [c for c in claude if any(k in c[2] for k in picks)]
        first = min([c[:2] + ("codex",) for c in cx] + [c[:2] + ("claude",) for c in cl], default=(None, None, None))
        out.append(dict(file=f"{lab_cache}/{name}", sha256=sha(path), author_session=first[2], first_at=first[0], model=first[1], codex_calls=len(cx), claude_calls=len(cl)))
    return out


def agents_of(weights, name, seen=None):
    """Transitive closure of label origins behind one set of weights, fit and tuning separately."""
    seen = seen if seen is not None else set()
    w = weights[name]
    agents, classes, fit_all, tune_all, bases = Counter(), Counter(), set(), set(w["tune"]), set(w["base"])
    for d in w["fit"]:
        fit_all.add(d)
    for parent in w["parents"]:
        if parent in seen:
            continue
        seen.add(parent)
        sub = agents_of(weights, parent, seen)
        fit_all |= set(sub["fit_closure"])
        tune_all |= set(sub["tune_closure"])
        bases |= set(sub["base_closure"])
    for d in fit_all | tune_all:
        key = d.split(":")[0]
        if key in DATASETS:
            agents[DATASETS[key][3]] += 1
            classes[DATASETS[key][2]] += 1
        elif key in TEMPLATES:
            agents[CODEX] += 1
            classes[PROGRAM] += 1
    return dict(fit_closure=sorted(fit_all), tune_closure=sorted(tune_all), base_closure=sorted(bases), label_agents=sorted(agents), label_classes=sorted(classes))


def verify_v2_hashes():
    """Each saved v2 run must name the same training file in experiments.json as this matrix."""
    records = json.loads((LAB / "v2/snapshot/experiments.json").read_text())["records"]
    by_hash = {sha(p): p.name for p in (CACHE / V2).glob("*.jsonl")}
    bad, checked = [], 0
    expected = dict((k, DATASETS[v][1][1]) for k, v in E_ROUNDS.items())
    expected["E18-goal-conditioned"] = "train-goals.jsonl"
    for r in records:
        if r["name"] in expected:
            checked += 1
            if by_hash.get(r.get("train_hash")) != expected[r["name"]]:
                bad.append(r["name"])
    return dict(checked=checked, mismatches=bad)


def verify_reuse():
    a, b = CACHE / V5 / "dev-known.jsonl", CACHE / V4 / "final-audited.jsonl"
    ids = lambda p: {json.loads(x)["id"] for x in p.read_text().splitlines() if x.strip()}
    known = ids(CACHE / V5 / "train-known.jsonl")
    v4 = ids(CACHE / V4 / "train-audited.jsonl") | ids(CACHE / V4 / "dev-audited.jsonl")
    return dict(v5_dev_known_equals_v4_final=sha(a) == sha(b), v5_train_known_rows=len(known), v4_train_plus_dev_rows=len(v4))


def app_integration():
    """No tracked weights anywhere, and no reference to a model runtime or lab outside research and docs."""
    files = subprocess.run(["git", "ls-files"], cwd=REPO, capture_output=True, text=True, check=True).stdout.splitlines()
    weights = [f for f in files if re.search(r"\.(onnx|pt|joblib|safetensors|npz|gguf)$", f)]
    pattern = re.compile(r"\bonnx|joblib|frx-ai-rnd|research/frx_ai|runtime-retrieval|sentence-transformers|safetensors|\bort::")
    refs = []
    for f in files:
        if f.startswith(("research/", "docs/")) or f == "research/frx_ai/audit/provenance_audit.py":
            continue
        p = REPO / f
        if not p.is_file() or p.stat().st_size > 2_000_000:
            continue
        try:
            if pattern.search(p.read_text()):
                refs.append(f)
        except (UnicodeDecodeError, OSError):
            continue
    return dict(tracked_weight_files=weights, app_references=refs)


def review_queues():
    out = {}
    for name, path in (("v2", LAB / "v2/snapshot/human-review-queue.json"), ("v4", CACHE / V4 / "human-review-queue.json"), ("v6", LAB / "v6/snapshot/human-review-queue.json")):
        rows = json.loads(path.read_text())
        out[name] = dict(open_items=len(rows), rows_with_a_review_answer=sum(1 for r in rows if any(k in r for k in ("review", "reviewer", "human_label", "adjudication"))))
    return out


def d7_rules():
    rows = [json.loads(x) for x in (CACHE / V7 / "d7.jsonl").read_text().splitlines() if x.strip()]
    return dict(rows=len(rows), splits=dict(Counter(r["split"] for r in rows)), label_authority="regex rules in v7/demos.py; the text is public maintainer speech")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True, help="explicit output path; use a private directory for session metadata")
    a = ap.parse_args()
    codex, meta, plans, users = codex_calls()
    claude, models, entry = claude_calls()
    datasets = {}
    for k, (lab, loc, origin, agent, how, use) in DATASETS.items():
        path = CACHE.joinpath(*loc)
        datasets[k] = dict(lab=lab, file="/".join(loc), origin=origin, agent=agent, how=how, use=use, independent_human_review=False,
                           rows=count(path), sha256=sha(path))
    rows = [json.loads(x) for x in (CACHE / V6 / "training-pairs.jsonl").read_text().splitlines()]
    pairs = Counter(("synthetic-text" if r["source_origin"].startswith("explicit-synthetic") else "real-text") + " / " + r["origin"] for r in rows)
    datasets["v6-training-pairs"]["composition"] = dict(pairs)
    weights = {}
    for k, w in WEIGHTS.items():
        path = CACHE.joinpath(*w["path"])
        weights[k] = dict(w, path="/".join(w["path"]), bytes=path.stat().st_size, sha256=sha(path), **agents_of(WEIGHTS, k))
    claude_before_v7 = [c for c in claude if c[0] < "2026-10-30T22:40" and re.search(r"frx-ai-rnd-(v[2-6]-)?2026", c[2])]
    result = dict(
        scope="v1-v7, read only; no training",
        access=dict(
            codex=dict(session=CODEX_SESSION.name, meta=meta, plan_type_events=plans,
                       note="plan_type comes from rate-limit events in the session; auth.json was not opened"),
            claude=dict(session=CLAUDE_SESSION.name, models=models, entrypoints=entry,
                        note="account type comes from non-secret fields of ~/.claude.json; credentials were not opened")),
        annotator_authorship=authorship(codex, claude),
        claude_calls_touching_v1_v6_caches_before_first_audit_read=len(claude_before_v7),
        human_review_queues=review_queues(),
        checks=dict(v2_training_hashes=verify_v2_hashes(), reuse=verify_reuse(), app=app_integration()),
        d7=d7_rules(), datasets=datasets, templates=TEMPLATES, weights=weights,
        upstream_weights_outside_this_audit=[M, "nli-deberta-v3-xsmall", "spaCy en_core_web_sm", "Stanza en"])
    a.out.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps(dict(
        datasets=len(datasets), weights=len(weights),
        annotators_by_session=Counter(x["author_session"] for x in result["annotator_authorship"]),
        v2_hashes=result["checks"]["v2_training_hashes"], reuse=result["checks"]["reuse"], app=result["checks"]["app"],
        queues=result["human_review_queues"], claude_touching=len(claude_before_v7), plans=plans, meta=meta,
        closure_agents=Counter(" + ".join(w["label_agents"]) or "none" for w in weights.values())), indent=1, default=str))


if __name__ == "__main__":
    main()
