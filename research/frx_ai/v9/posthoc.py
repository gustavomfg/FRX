"""Post-hoc diagnosis of the frozen reading: where each answer was lost. Known data; changes no number."""
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import net  # noqa: E402
import reading as rd  # noqa: E402
import sources  # noqa: E402

CACHE = net.CACHE


def read(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]


def main():
    first = json.loads((CACHE / "first-reading.json").read_text())
    cases = read(CACHE / "test-reserved.jsonl")
    gold = {g["id"]: g for g in read(CACHE / "test-gold.jsonl")}
    tool = sources.Frozen()
    rows = []
    for case in cases:
        g = gold[case["id"]]
        if not g["documented"]:
            continue
        run = next(r for r in first["rows"] if r["id"] == case["id"] and r["condition"] == "C")
        best = dict(threads=0, concept=None, with_context=0)
        facts = rd.interpret(case["input"])
        context = facts["phenomena"][:1] or []
        for concept in g["concepts"]:
            found = tool.search([concept], "ValveSoftware/Proton", case["cutoff"], exclude=set(case["exclude"]))
            near = tool.search([concept] + context, "ValveSoftware/Proton", case["cutoff"], exclude=set(case["exclude"])) if context else dict(total=0)
            if found["total"] > best["threads"]:
                best = dict(threads=found["total"], concept=concept, with_context=near["total"])
        stage = "in report" if run["report_concepts"] else "read, not in report" if run["read_concepts"] else "in the corpus, not read" if best["threads"] else "not in the corpus before the date"
        rows.append(dict(id=case["id"], stratum=case["stratum"], concepts=g["concepts"], stage=stage, **best, queries=[" ".join(q["terms"]) for q in run["gain"]], applicable=run["applicable"], rejected=run["rejected"]))
    follow = [q for r in first["rows"] if r["condition"] == "C" for q in r["gain"][1:]]
    single_word = [q for r in first["rows"] if r["condition"] == "C" for q in r["gain"] if len(q["terms"]) == 1]
    counts = {}
    for r in rows:
        counts[r["stage"]] = counts.get(r["stage"], 0) + 1
    kinds = {}
    for q in follow:
        k = kinds.setdefault(q["kind"], dict(queries=0, no_new_threads=0, new_leads=0, new_concept=0))
        k["queries"] += 1
        k["no_new_threads"] += not q["new_threads"]
        k["new_leads"] += bool(q["new_leads"])
        k["new_concept"] += bool(q["new_concepts"])
    out = dict(status="post-hoc on known data; H0 gold", stages=counts, rows=rows, followups_by_kind=kinds, single_word_queries=len(single_word), first_query_empty=sum(1 for r in first["rows"] if r["condition"] == "C" and r["gain"] and not r["gain"][0]["new_threads"]))
    (CACHE / "posthoc.json").write_text(json.dumps(out, indent=1))
    print(json.dumps({k: v for k, v in out.items() if k != "rows"}, indent=0))
    for r in rows:
        print(r["id"], r["stage"], r["concept"], r["threads"], r["with_context"])


if __name__ == "__main__" and "--stages" in sys.argv:
    main()


# Known replays: each changes one suspected bottleneck. They are diagnosis, never a second test.
def or_search(tool, text, cutoff, exclude, limit=6):
    """Classic retrieval over the whole problem text: any term may match, rare terms weigh more."""
    import math
    total = len(tool.issues)
    words = {}
    for t in set(sources.tokens(text)):
        docs = tool.post.get(t)
        if docs and len(t) > 2:
            words[t] = math.log(1 + (total - len(docs) + .5) / (len(docs) + .5))
    scores = {}
    for t in sorted(words, key=lambda x: -words[x])[:14]:
        for n, (tf, first) in tool.post[t].items():
            if first < cutoff and n not in exclude and tool.issues[n]["created_at"] < cutoff:
                scores[n] = scores.get(n, 0) + words[t] * tf * 2.2 / (tf + 1.2 * (.25 + .75 * tool.lengths[n] / tool.avg))
    return sorted(scores, key=lambda n: (-scores[n], n))[:limit]


def score_state(state, concepts):
    import engine
    result = engine.report(state)
    text = "\n".join(f["term"] + " " + " ".join(e["quote"] for e in f["evidence"]) for f in result["findings"]).lower()
    read_text = "\n".join(p["full_text"] for p in state.passages).lower()
    return bool([c for c in concepts if c in text]), bool([c for c in concepts if c in read_text]), result["status"]


def rare_terms(tool, maximum=120):
    """Terms unknown to the hand-written vocabulary: rare words of the corpus itself. Counting, not training."""
    original = rd.terms_of

    def extended(text):
        out = original(text)
        for w in set(sources.tokens(re.sub(r"https?://\S+", " ", text or ""))):
            docs = tool.post.get(w)
            if docs and 2 <= len(docs) <= maximum and len(w) >= 4 and not w.isdigit() and w not in out and w not in rd.GENERIC and not re.fullmatch(r"[0-9a-f]{8,}|\d[\d.]*", w):
                out[w] = "component"
        return out
    return original, extended


def replays():
    import engine
    first_cases = read(CACHE / "test-reserved.jsonl")
    gold = {g["id"]: g for g in read(CACHE / "test-gold.jsonl")}
    tool = sources.Frozen()
    cases = [c for c in first_cases if gold[c["id"]]["documented"]]
    tally = {}
    per = {}

    def note(name, cid, outcome):
        t = tally.setdefault(name, dict(report=0, read=0, insufficient=0))
        t["report"] += outcome[0]
        t["read"] += outcome[1]
        t["insufficient"] += outcome[2] == "insufficient-evidence"
        per.setdefault(name, {})[cid] = [outcome[0], outcome[1]]

    def read_all(state, numbers):
        for n in numbers[:6]:
            engine.absorb(state, engine.do_read(state, tool, "ValveSoftware/Proton", n, dict(terms=[])))

    original, extended = rare_terms(tool)
    for case in cases:
        concepts = gold[case["id"]]["concepts"]
        exclude = set(case["exclude"])
        text = case["input"]["symptom"] + " " + (case["input"].get("title") or "")
        for name, patched in (("P1 single search over the whole problem text", False), ("P4 P1 plus rare corpus terms", True)):
            rd.terms_of = extended if patched else original
            state = engine.State(case["input"], case["cutoff"])
            read_all(state, or_search(tool, text, case["cutoff"], exclude))
            note(name, case["id"], score_state(state, concepts))
        rd.terms_of = extended
        state = engine.investigate(case["input"], tool, "iterative", until=case["cutoff"], exclude=exclude)
        note("P3 iterative with rare corpus terms as leads", case["id"], score_state(state, concepts))
        rd.terms_of = original
        # Ceiling: the answer's own words as the query. No real system can do this.
        state = engine.State(case["input"], case["cutoff"])
        numbers = []
        for concept in concepts:
            numbers += [h["number"] for h in tool.search([concept], "ValveSoftware/Proton", case["cutoff"], exclude=exclude)["hits"][:3]]
        read_all(state, list(dict.fromkeys(numbers)))
        note("P2 oracle: query made of the answer itself", case["id"], score_state(state, concepts))
    rd.terms_of = original
    return dict(status="known replays after the first reading; diagnosis only", documented=len(cases), replays=tally, per_case=per)


if __name__ == "__main__" and "--stages" not in sys.argv:
    out = replays()
    (CACHE / "replays.json").write_text(json.dumps(out, indent=1))
    print(json.dumps(out["replays"], indent=0))
