"""Compares strategies on the frozen snapshot, cut at each case's date. H0 gold: not human validation."""
import argparse
import hashlib
import importlib.util
import json
import random
import resource
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import engine  # noqa: E402
import freeze9 as freeze  # noqa: E402
import net  # noqa: E402
import reading as rd  # noqa: E402
import sources  # noqa: E402

CACHE = net.CACHE
STATUS = "H0: gold by the agent, not human validation"
STRATEGIES = dict(A="local", B="single", B2="single-broad", F="fixed", R="random", C="iterative", S="iterative")
ONLINE = ("B", "B2", "F", "R", "C", "S")


class Encoder:
    """The public MiniLM, frozen and without FRX labels, used only to reorder search results."""

    def __init__(self):
        file = HERE.parent / "v2/retrieval_runtime.py"
        spec = importlib.util.spec_from_file_location("v9_frozen_retriever", file)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.model = module.Retriever(Path.home() / ".cache/frx-ai-rnd-v2-20261029/runtime-frozen-control")

    def rerank(self, query, hits):
        vectors = self.model.encode([query[:1000]] + [(h["title"] + " " + h.get("snippet", ""))[:1000] for h in hits])
        scores = vectors[1:] @ vectors[0]
        order = sorted(range(len(hits)), key=lambda i: -float(scores[i]))
        return [hits[i] for i in order]


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def contains(text, concepts):
    low = (text or "").lower()
    return [c for c in concepts if c.lower() in low]


def report_text(result):
    parts = [f["term"] + " " + " ".join(e["quote"] for e in f["evidence"]) for f in result["findings"]]
    return "\n".join(parts)


def local_text(result):
    return "\n".join(h["id"].replace("-", " ") for h in result["catalogue_hypotheses"]) + "\n" + "\n".join(c["question"] + " " + c["how"] for c in result["next_checks"])


def verify_citations(tool, result, until):
    """Each quote must exist in the part it cites, and the hash must match that part."""
    checked = literal = hashed = 0
    for f in result["findings"]:
        for e in f["evidence"]:
            checked += 1
            try:
                doc = tool.read(e["repo"], e["thread"], until)
            except net.NotFound:
                continue
            tool.requests -= 1
            part = next((p for p in doc["parts"] if p["id"] == e["part_id"]), None)
            if part:
                literal += e["quote"] in rd.sanitize(part["text"])
                hashed += part["sha256"] == e["sha256"]
    return dict(checked=checked, literal=literal, hash_ok=hashed)


def run_case(case, name, tool, encoder):
    start = time.perf_counter()
    state = engine.State(case["input"], case["cutoff"])
    state.semantic = name == "S"
    state = engine.investigate(case["input"], tool, STRATEGIES[name], until=case["cutoff"], exclude=set(case["exclude"]), encoder=encoder if name == "S" else None, prior=state)
    result = engine.report(state, STRATEGIES[name])
    seconds = time.perf_counter() - start
    concepts = case["gold"].get("concepts", [])
    text = local_text(result) if name == "A" else report_text(result)
    read_text = "\n".join(p["full_text"] for p in state.passages)
    gain = []
    seen = set()
    for i, q in enumerate(state.queries):
        passages = [p for p in state.passages if p.get("via_query") == i]
        new = set(contains("\n".join(p["full_text"] for p in passages), concepts)) - seen
        seen |= new
        gain.append(dict(kind=q["kind"], terms=q["terms"], new_threads=q.get("new_threads", 0), new_leads=len(q.get("new_leads") or []), new_concepts=sorted(new), lead=q.get("lead")))
    docs = sorted({(p["repo"], p["thread"], p["title"]) for p in state.passages})
    return dict(id=case["id"], condition=name, stratum=case["stratum"], documented=case["gold"]["documented"], report_concepts=contains(text, concepts), read_concepts=contains(read_text, concepts), status=result["status"],
                top3=[c["id"] for c in result["next_checks"]], top3_ok=bool(set(c["id"] for c in result["next_checks"]) & set(case["gold"].get("checks", []))), searches=len(state.queries), reads=len(state.seen), passages=len(state.passages), applicable=result["applicable_passages"], rejected=result["rejected_passages"], contradictions=[c["term"] for c in result["contradictions"]],
                findings=[dict(term=f["term"], kind=f["kind"], status=f["status"], threads=f["threads"], positive=f["positive"], negative=f["negative"], quote=f["evidence"][0]["quote"][:220] if f["evidence"] else "", url=f["evidence"][0]["url"] if f["evidence"] else None, applicability=f["evidence"][0]["applicability"] if f["evidence"] else None) for f in result["findings"]],
                gain=gain, docs=[dict(repo=r, thread=t, title=x) for r, t, x in docs], seconds=round(seconds, 3), citations=verify_citations(tool, result, case["cutoff"]) if name != "A" else None, failures=result["failures"], steps=result["steps"], confirmed_causes=result["confirmed_causes"])


def attach_gold(cases, gold):
    by = {g["id"]: g for g in gold}
    for c in cases:
        c["gold"] = by[c["id"]]
    return cases


def run_all(cohort, conditions=tuple(STRATEGIES)):
    cases = attach_gold(read(CACHE / f"{cohort}-reserved.jsonl"), read(CACHE / f"{cohort}-gold.jsonl"))
    tool = sources.Frozen()
    encoder = Encoder() if "S" in conditions else None
    return [run_case(c, n, tool, encoder) for c in cases for n in conditions]


def paired(rows, a, b, key, n=5000):
    ids = sorted({r["id"] for r in rows if r["documented"]})
    get = lambda name: {r["id"]: float(bool(r[key])) for r in rows if r["condition"] == name}
    x, y = get(a), get(b)
    d = [x[i] - y[i] for i in ids]
    rng = random.Random(f"frx-v9/{a}/{b}/{key}")
    means = sorted(sum(d[rng.randrange(len(d))] for _ in d) / len(d) for _ in range(n))
    return dict(difference=round(sum(d) / len(d), 3), ci95=[round(means[int(.025 * n)], 3), round(means[int(.975 * n)], 3)], wins=sum(v > 0 for v in d), losses=sum(v < 0 for v in d), cases=len(d))


def summary(rows):
    out = {}
    for name in STRATEGIES:
        r = [x for x in rows if x["condition"] == name]
        doc = [x for x in r if x["documented"]]
        undoc = [x for x in r if not x["documented"]]
        cites = [x["citations"] for x in r if x["citations"]]
        follow = [g for x in r for g in x["gain"][1:]]
        out[name] = dict(cases=len(r), documented=len(doc), report_concept=sum(bool(x["report_concepts"]) for x in doc), read_concept=sum(bool(x["read_concepts"]) for x in doc), top3_ok=sum(x["top3_ok"] for x in r), mean_searches=round(sum(x["searches"] for x in r) / max(1, len(r)), 2), mean_reads=round(sum(x["reads"] for x in r) / max(1, len(r)), 2),
                         abstained_undocumented=[sum(x["status"] == "insufficient-evidence" for x in undoc), len(undoc)], abstained_documented=[sum(x["status"] == "insufficient-evidence" for x in doc), len(doc)], contradiction_cases=sum(bool(x["contradictions"]) for x in r), citations=dict(checked=sum(c["checked"] for c in cites), literal=sum(c["literal"] for c in cites), hash_ok=sum(c["hash_ok"] for c in cites)),
                         followup_queries=len(follow), followups_no_new_threads=sum(not g["new_threads"] for g in follow), followups_new_concept=sum(bool(g["new_concepts"]) for g in follow), mean_seconds=round(sum(x["seconds"] for x in r) / max(1, len(r)), 3))
    pairs = {f"{a}-{b}": {k: paired(rows, a, b, k) for k in ("report_concepts", "read_concepts")} for a, b in (("C", "B"), ("C", "B2"), ("C", "F"), ("C", "R"), ("C", "A"), ("S", "C"), ("B", "A"), ("F", "B"))}
    return dict(status=STATUS, by_condition=out, paired=pairs, max_rss_mb=round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024, 1))


def judge_export():
    """One shuffled list of read documents per case, with no sign of which condition read them."""
    data = json.loads((CACHE / "first-reading.json").read_text())
    cases = {c["id"]: c for c in read(CACHE / "test-reserved.jsonl")}
    tool = sources.Frozen()
    rows = []
    for cid, case in cases.items():
        pool = {(d["repo"], d["thread"]): d for r in data["rows"] if r["id"] == cid and r["condition"] in ONLINE for d in r["docs"]}
        keys = sorted(pool)
        random.Random(f"{cid}/pool").shuffle(keys)
        rows.append(dict(id=cid, game=case["input"]["game"], symptom=case["input"]["symptom"][:260], gpu=case["input"]["gpu"], docs=[dict(thread=k[1], title=pool[k]["title"][:80]) for k in keys]))
    (CACHE / "judge-pool.json").write_text(json.dumps(rows, indent=0))
    print("documents", sum(len(r["docs"]) for r in rows))


def judged():
    """Precision of read documents per condition, from blind labels: 2 same problem and context, 1 related technique, 0 unrelated."""
    data = json.loads((CACHE / "first-reading.json").read_text())
    labels = json.loads((CACHE / "judgments.json").read_text())
    out = {}
    for name in ONLINE:
        rel = app = n = 0
        for r in data["rows"]:
            if r["condition"] != name:
                continue
            for d in r["docs"]:
                v = labels[r["id"]].get(str(d["thread"]), 0)
                n += 1
                rel += v >= 1
                app += v == 2
        out[name] = dict(documents=n, relevant=rel, applicable=app)
    return out


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("command", choices=["dev", "lock", "first", "judge-export", "judged"])
    a = p.parse_args()
    if a.command == "dev":
        rows = run_all("dev")
        (CACHE / "dev-rows.json").write_text(json.dumps(rows, indent=1))
        print(json.dumps(summary(rows)["by_condition"], indent=0))
    elif a.command == "lock":
        dest = CACHE / "label-lock.json"
        if dest.exists():
            raise SystemExit("labels already locked")
        dest.write_text(json.dumps(dict(sha256=sha(CACHE / "test-gold.jsonl")), indent=1))
    elif a.command == "first":
        changed = freeze.changed()
        if changed:
            raise SystemExit(f"frozen code changed: {changed}")
        if sha(CACHE / "test-gold.jsonl") != json.loads((CACHE / "label-lock.json").read_text())["sha256"]:
            raise SystemExit("labels changed after the lock")
        dest = CACHE / "first-reading.json"
        if dest.exists():
            raise SystemExit("preserve the first reading")
        rows = run_all("test")
        out = dict(rows=rows, summary=summary(rows), frozen=json.loads((CACHE / "frozen.json").read_text())["frozen_at"])
        with dest.open("x") as f:
            json.dump(out, f, indent=1)
        print(json.dumps(out["summary"], indent=0))
    elif a.command == "judge-export":
        judge_export()
    else:
        print(json.dumps(judged(), indent=1))
