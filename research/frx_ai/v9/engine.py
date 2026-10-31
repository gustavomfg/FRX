"""The investigation loop: formulate, search, read, evaluate, update, and decide what next.

Every decision here is an explicit rule, except the optional semantic reranking (frozen MiniLM).
No AI service is called. External text can only become data: a query term in a restricted format,
or a quotation. The next check comes from the project's own catalogue, never from a document.
"""
import random
import re
import sys
import time
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import net  # noqa: E402
import reading as rd  # noqa: E402

sys.path.insert(0, str(HERE.parent / "v8"))
import investigator as catalog  # noqa: E402

KIND_WEIGHT = dict(library=3.0, option=3.0, error=3.0, component=2.0, version=.7)
NATURE_WEIGHT = {"official-document": 1.5, "maintainer-statement": 1.5, "reported-result-maintainer": 1.5, "reported-result-community": 1.2, "attempt": 1.0, "community-information": .7, "user-hypothesis": .5}
APPLICABLE = {"same-game", "technical-analogy"}
ROUTES = [(re.compile(r"(?i)dxvk|d3d(?:9|10|11)|dxgi"), "doitsujin/dxvk"), (re.compile(r"(?i)vkd3d|d3d12"), "HansKristian-Work/vkd3d-proton"), (re.compile(r"(?i)protonfix|umu"), "Open-Wine-Components/umu-protonfixes")]
PRIMARY = "ValveSoftware/Proton"
CHECK_FOR_KIND = dict(library="inspect-import-chain", option="review-launch-options", error="locate-faulting-module", version="test-other-proton")
CHECK_FOR_COMPONENT = {"physx": "check-physx-chain", "mesa": "check-driver-stack", "radv": "check-driver-stack", "nvk": "check-driver-stack", "amdgpu": "read-kernel-gpu-messages", "vulkan": "check-vulkan-support", "dxvk": "check-vulkan-support", "vkd3d": "check-vulkan-support", "vkd3d-proton": "check-vulkan-support", "mfplat": "inspect-media-errors", "mf": "inspect-media-errors", "winegstreamer": "inspect-media-errors", "gstreamer": "inspect-media-errors", "h264": "inspect-media-errors", "faudio": "inspect-audio-route", "pipewire": "inspect-audio-route", "pulseaudio": "inspect-audio-route", "alsa": "inspect-audio-route", "xact": "inspect-audio-route", "easyanticheat": "check-anticheat-support", "eac": "check-anticheat-support", "battleye": "check-anticheat-support", "denuvo": "check-anticheat-support", "crackproof": "check-anticheat-support", "sdl": "inspect-input-route", "hidraw": "inspect-input-route", "dualsense": "inspect-input-route", "steam-input": "inspect-input-route", "xinput": "inspect-input-route", "pressure-vessel": "compare-runtime-host", "sniper": "compare-runtime-host", "soldier": "compare-runtime-host", "steam-runtime": "compare-runtime-host"}

BUDGETS = dict(searches=3, reads=6)
PRESENT = re.compile(r"(?i)\b[\w.+-]+\.(?:dll|so(?:\.\d+)*)\b[^.\n]{0,40}\b(?:exists?|is\s+(?:there|present|installed)|present|found\s+it)\b")
FIXED_TEMPLATES = (("game", "symptom"), ("game", "proton"), ("symptom", "proton", "linux"))


def route(term):
    for rx, repo in ROUTES:
        if rx.search(term):
            return repo
    return PRIMARY


def game_query(facts):
    tokens = facts["game_tokens"][:3]
    return " ".join(tokens) if tokens else None


def symptom_word(facts):
    for s in facts["symptoms"]:
        if s in rd.SYMPTOM_WORD:
            return rd.SYMPTOM_WORD[s]
    return None


def initial_query(facts):
    """Game and strongest symptom, plus any exact error or library already given."""
    specific = [t for t, k in facts["input_terms"].items() if k in ("library", "error", "option") and rd.safe_term(t)]
    components = [t for t, k in facts["input_terms"].items() if k == "component" and rd.safe_term(t)]
    query = [x for x in [game_query(facts)] if x]
    query += specific[:1] or components[:1] or facts["phenomena"][:1] or [x for x in [symptom_word(facts)] if x]
    if not game_query(facts):
        # Without a game, one word is too broad: add the next observed phenomenon.
        query += [p for p in facts["phenomena"] if p not in query][:1]
    return dict(terms=_flat(query), kind="initial", reason="jogo e sintoma ou termo técnico fornecidos", repo=PRIMARY)


def broad_query(facts):
    """No usable lead: leave the game aside and search the technical picture instead."""
    parts = [t for t, k in facts["input_terms"].items() if k in ("library", "error", "option") and rd.safe_term(t)][:1]
    parts += [t for t, k in facts["input_terms"].items() if k == "component" and rd.safe_term(t)][:1]
    parts += facts["phenomena"][:2 - min(2, len(parts))] if len(parts) < 2 else []
    vendor = facts["known"].get("gpu_vendor")
    if vendor and len(parts) < 3 and parts:
        parts.append(vendor)
    if len(_flat(parts)) < 2:
        return None
    return dict(terms=_flat(parts), kind="broadening", reason="sem pistas: pesquisar o quadro técnico, sem o nome do jogo", repo=PRIMARY)


def _flat(parts):
    return [w for p in parts for w in p.split()]


def fixed_query(facts, template):
    """Queries from the problem alone. They never use what was read."""
    values = dict(game=game_query(facts), symptom=(facts["phenomena"][:1] or [symptom_word(facts)])[0], proton="proton", linux="linux")
    parts = [values[t] for t in template if values.get(t)]
    return dict(terms=_flat(parts), kind="fixed-template", reason="modelo fixo: não usa o que foi lido", repo=PRIMARY) if parts else None


class State:
    """Case memory: only this investigation. Nothing here reaches any model parameter."""

    def __init__(self, problem, until=None):
        self.facts = rd.interpret(problem)
        self.facts["today"] = until or time.strftime("%Y-%m-%d")
        self.until = until
        self.passages, self.seen, self.queries, self.steps, self.failures, self.refs = [], set(), [], [], [], []
        self.leads = defaultdict(lambda: dict(score=0.0, threads=set(), natures=set(), kind=None, positive=0, negative=0, passages=[], same_game=0))
        self.results = []

    def query_key(self, q):
        return " ".join(sorted(t.lower() for t in q["terms"]))

    def used_terms(self):
        return {t.lower() for q in self.queries for t in q["terms"]}


def absorb(state, passages):
    """Update leads from what applies. Hostile or incompatible passages feed no lead."""
    before = set(state.leads)
    for p in passages:
        state.passages.append(p)
        if p["hostile"] or p["applicability"]["status"] not in APPLICABLE:
            continue
        weight = NATURE_WEIGHT.get(p["nature"], .7) * (1.5 if p["applicability"]["status"] == "same-game" else 1.0)
        positive = sum(c["cls"] in ("resolved", "improved") for c in p["claims"])
        negative = sum(c["cls"] in ("not_resolved", "worsened") for c in p["claims"])
        for term, kind in p["terms"].items():
            if term in state.facts["input_terms"] or term in rd.GENERIC:
                continue
            lead = state.leads[term]
            lead["kind"] = kind
            # Several threads are worth more than repetition inside one.
            bonus = 1.0 if p["thread"] not in lead["threads"] else .25
            lead["score"] += KIND_WEIGHT.get(kind, 1.0) * weight * bonus * (1.5 if p["claims"] else 1.0)
            lead["threads"].add(p["thread"])
            lead["natures"].add(p["nature"])
            lead["positive"] += positive
            lead["negative"] += negative
            lead["same_game"] += p["applicability"]["status"] == "same-game"
            lead["passages"].append(len(state.passages) - 1)
        for ref in p["references"]:
            if ref[0] in net.REPOSITORIES and ref not in state.refs:
                state.refs.append(ref)
    return sorted(set(state.leads) - before)


def next_query(state, chooser=None):
    """Decide the next search from evidence. Returns None when nothing new is worth asking."""
    used = state.used_terms()
    open_leads = [(t, v) for t, v in state.leads.items() if t.lower() not in used and rd.safe_term(t.replace("-", " ") if v["kind"] == "version" else t)]
    if not open_leads:
        # Every lead was searched: the best one with only favourable reports gets a contradiction search.
        tested = [(t, v) for t, v in state.leads.items() if v["positive"] and not v["negative"]]
        tested = sorted(tested, key=lambda x: -x[1]["score"])
        symptom = symptom_word(state.facts)
        for t, v in tested:
            q = dict(terms=[t, "regression"], kind="contradiction", reason=f"relatos só favoráveis sobre {t}: procurar falhas", repo=route(t), lead=t)
            if state.query_key(q) not in {state.query_key(x) for x in state.queries} and rd.safe_term(t):
                return q
        broad = broad_query(state.facts)
        if broad and state.query_key(broad) not in {state.query_key(x) for x in state.queries}:
            return broad
        return None
    if chooser == "random":
        rng = random.Random(f"{state.facts['known'].get('game')}/{len(state.queries)}")
        term, lead = rng.choice(sorted(open_leads, key=lambda x: x[0]))
    else:
        term, lead = max(open_leads, key=lambda x: (x[1]["score"], x[0]))
    text = term.replace("-", " ") if lead["kind"] == "version" else term
    vendor = state.facts["known"].get("gpu_vendor")
    tokens = state.facts["game_tokens"][:2]
    # Same game first: then the same lead in other discussions; then the environment.
    if lead["positive"] and lead["negative"]:
        kind, reason, extra = "verification", f"{term} tem relatos a favor e contra", [symptom_word(state.facts)]
    elif lead["same_game"] and tokens and len(state.queries) == 1:
        kind, reason, extra = "specialized", f"{term} apareceu em evidência aplicável", []
    elif vendor and len(state.queries) >= 2:
        kind, reason, extra = "contextual", f"{term} se aplica a {vendor}?", [vendor]
    else:
        kind, reason, extra = "specialized", f"{term} apareceu em evidência aplicável", [symptom_word(state.facts)]
    return dict(terms=_flat([text] + [x for x in extra if x]), kind=kind, reason=reason, repo=route(term), lead=term)


def rank_hits(state, hits, encoder=None, query=None):
    """Unread hits first; same game gets priority. Reranking only reorders what the search returned."""
    fresh = [h for h in hits if (h["repo"], h["number"]) not in state.seen]
    def key(h):
        return (0 if rd.same_game(h["title"], state.facts) else 1, -(h.get("score") or 0))
    ordered = sorted(fresh, key=key)
    if encoder is not None and ordered:
        text = " ".join(query["terms"]) + " " + (state.facts["known"].get("symptom") or "")[:300]
        ordered = encoder.rerank(text, ordered)
    return ordered


def do_search(state, tool, q, exclude):
    start = len(state.queries)
    state.queries.append(q)
    try:
        answer = tool.search(q["terms"], q["repo"], state.until, exclude=exclude)
    except net.RateLimited as e:
        state.failures.append(dict(action="search", query=q["terms"], reason="rate-limited", reset=e.reset))
        return None
    except (net.Unavailable, net.Blocked, net.NotFound) as e:
        state.failures.append(dict(action="search", query=q["terms"], reason=str(e)[:120]))
        return None
    q["total"] = answer.get("total", 0)
    q["returned"] = len(answer.get("hits", []))
    q["index"] = start
    return answer


def do_read(state, tool, repo, number, q):
    state.seen.add((repo, number))
    try:
        doc = tool.read(repo, number, state.until)
    except net.RateLimited as e:
        state.failures.append(dict(action="read", thread=number, reason="rate-limited", reset=e.reset))
        return []
    except (net.Unavailable, net.Blocked, net.NotFound) as e:
        state.failures.append(dict(action="read", thread=number, reason=str(e)[:120]))
        return []
    found = rd.read_document(doc, state.facts, q["terms"], state.until)
    for p in found:
        p["via_query"] = q.get("index", -1)
    return found


def relax(q, facts):
    """Lexical search needs every term. On zero results, keep the technical part and drop the game."""
    if len(q["terms"]) <= 1:
        return None
    game = set(facts["game_tokens"])
    technical = [t for t in q["terms"] if t.lower() not in game]
    terms = technical if technical and len(technical) < len(q["terms"]) else q["terms"][:-1]
    return dict(q, terms=terms, kind=q["kind"], reason=q["reason"] + "; sem resultados, consulta reduzida", relaxed=True)


def investigate(problem, tool, strategy="iterative", until=None, exclude=(), encoder=None, on_step=None, budgets=BUDGETS, prior=None):
    """Strategies: local, single, fixed, random, iterative. Same reader and evaluator for all."""
    state = prior or State(problem, until)
    say = on_step or (lambda *_: None)
    say("understand", dict(known={k: v for k, v in state.facts["known"].items() if v and k not in ("symptom",)}, unknown=state.facts["unknown"]))
    reads = searches = 0
    pending = []
    if strategy != "local":
        plan = []
        if strategy == "fixed":
            plan = [q for q in (fixed_query(state.facts, t) for t in FIXED_TEMPLATES) if q]
        elif strategy == "single":
            plan = [initial_query(state.facts)]
        elif strategy == "single-broad":
            plan = [q for q in [broad_query(state.facts) or initial_query(state.facts)] if q]
        chooser = "random" if strategy == "random" else None
        per_round = {"single": budgets["reads"], "single-broad": budgets["reads"], "fixed": 2, "random": 2, "iterative": 2}.get(strategy, 2)
        one = strategy in ("single", "single-broad")
        while searches < (1 if one else budgets["searches"]) and reads < budgets["reads"]:
            if one or strategy == "fixed":
                if not plan:
                    break
                q = plan.pop(0)
            elif searches == 0:
                q = initial_query(state.facts) if not state.queries else next_query(state, chooser)
            else:
                q = next_query(state, chooser)
            if q is None:
                state.steps.append(dict(decision="stop", reason="nenhuma pista nova para pesquisar"))
                break
            if not q["terms"]:
                state.steps.append(dict(decision="stop", reason="sem termos para formular a consulta"))
                break
            say("search", q)
            answer = do_search(state, tool, q, exclude)
            searches += 1
            if answer is None:
                if state.failures and state.failures[-1].get("reason") == "rate-limited":
                    state.steps.append(dict(decision="stop", reason="limite de taxa da fonte"))
                    break
                continue
            if not answer["hits"] and searches < budgets["searches"]:
                reduced = relax(q, state.facts)
                if reduced and not one:
                    say("reformulate", reduced)
                    answer = do_search(state, tool, reduced, exclude) or answer
                    searches += 1
                elif reduced:
                    # A single search keeps its one query: reducing it would be a second search.
                    pass
            ordered = rank_hits(state, answer["hits"], encoder, q)
            pending += [(h, q) for h in ordered]
            take = budgets["reads"] - reads if one else min(per_round, budgets["reads"] - reads)
            before_threads = {p["thread"] for p in state.passages}
            new_leads, new_passages = [], 0
            for h in ordered[:take]:
                say("read", dict(url=h["url"], title=rd.sanitize(h["title"])[:80]))
                found = do_read(state, tool, h["repo"], h["number"], q)
                reads += 1
                new_leads += absorb(state, found)
                new_passages += len(found)
            q["new_threads"] = len({p["thread"] for p in state.passages} - before_threads)
            q["new_leads"] = sorted(set(new_leads))
            q["new_passages"] = new_passages
            state.steps.append(dict(decision="searched", query=q["terms"], kind=q["kind"], lead=q.get("lead"), reason=q["reason"], new_threads=q["new_threads"], new_leads=len(q["new_leads"])))
            say("assess", dict(new_threads=q["new_threads"], new_leads=q["new_leads"][:8]))
            # Two searches in a row that add nothing end the investigation.
            recent = [x for x in state.queries if "new_threads" in x][-2:]
            if strategy in ("iterative", "random") and len(recent) == 2 and not any(x["new_threads"] or x["new_leads"] for x in recent):
                state.steps.append(dict(decision="stop", reason="duas consultas sem informação nova"))
                break
        # Search results not read yet use what remains of the same reading budget.
        if strategy in ("iterative", "random", "fixed"):
            refs = state.refs if strategy == "iterative" else []
            for h, q in pending:
                if reads >= budgets["reads"] - min(1, len(refs)):
                    break
                if (h["repo"], h["number"]) in state.seen:
                    continue
                say("read", dict(url=h["url"], title=rd.sanitize(h["title"])[:80]))
                absorb(state, do_read(state, tool, h["repo"], h["number"], q))
                reads += 1
        # Remaining reads follow references found in applicable evidence, when the strategy may adapt.
        if strategy == "iterative":
            for repo, number in list(state.refs):
                if reads >= budgets["reads"]:
                    break
                if (repo, number) in state.seen or number in exclude:
                    continue
                say("read", dict(url=f"{repo}#{number}", title="referência citada na evidência"))
                found = do_read(state, tool, repo, number, dict(terms=[]))
                reads += 1
                absorb(state, found)
                state.steps.append(dict(decision="followed-reference", thread=number, repo=repo))
    state.counts = dict(searches=len(state.queries), reads=len(state.seen))
    return state


def conclude(state, policy="discriminate"):
    """Provisional findings. Several sources mean more reports, not a confirmed cause."""
    good = [p for p in state.passages if not p["hostile"] and p["applicability"]["status"] in APPLICABLE]
    rejected = [p for p in state.passages if p["hostile"] or p["applicability"]["status"] == "incompatible"]
    findings = []
    for term, lead in sorted(state.leads.items(), key=lambda x: -x[1]["score"]):
        if lead["kind"] == "version" and lead["score"] < 2:
            continue
        chosen = [state.passages[i] for i in lead["passages"]]
        chosen.sort(key=lambda p: -len(p["claims"]))
        status = "contraditório" if lead["positive"] and lead["negative"] else "relatos favoráveis, não verificados" if lead["positive"] else "relatos negativos" if lead["negative"] else "mencionado em evidência aplicável"
        findings.append(dict(term=term, kind=lead["kind"], status=status, score=round(lead["score"], 2), threads=len(lead["threads"]), positive=lead["positive"], negative=lead["negative"], same_game_passages=lead["same_game"], natures=sorted(lead["natures"]), evidence=[_cite(p) for p in chosen[:2]], origin="rule"))
    top = findings[:6]
    contradictions = [f for f in top if f["status"] == "contraditório"]
    case = dict(game=state.facts["known"].get("game"), problem=state.facts["known"].get("symptom") or "", turns=[dict(role="reporter", text=state.facts["text"])], results=state.results, context={}, log=None)
    base = catalog.state_of(case)
    # Log the person already supplied: its signals count as evidence and asking again would be redundant.
    if state.facts["known"].get("log_supplied"):
        base["have"] = sorted(set(base["have"]) | {"log"})
    checks = catalog.POLICIES[policy](base)
    done = {r["check"] for r in state.results}
    ordered = []
    if findings:
        for f in top:
            cid = CHECK_FOR_COMPONENT.get(f["term"]) or CHECK_FOR_KIND.get(f["kind"])
            if cid and cid not in done and cid in catalog.CHECKS and cid not in [c["id"] for c in ordered]:
                c = catalog.CHECKS[cid]
                ordered.append(dict(id=cid, question=c["what"], how=c["how"], why=f"pista {f['term']} ({f['status']})", changes_state=bool(c.get("changes_state")), origin="rule"))
    for c in checks:
        if c["id"] not in [x["id"] for x in ordered] and c["id"] not in done:
            ordered.append(dict(id=c["id"], question=c.get("question", ""), how=c.get("how", ""), why="catálogo local", changes_state=bool(c.get("changes_state")), origin="rule"))
    # Reading comes before anything that changes the system.
    ordered.sort(key=lambda c: c["changes_state"])
    # Mentions alone do not make a conclusion: it takes a reported result or the same game.
    substantive = [f for f in findings if f["positive"] or f["negative"] or f["same_game_passages"]]
    insufficient = not good or not substantive
    status = "insufficient-evidence" if insufficient else "provisional"
    return dict(status=status, findings=top, contradictions=contradictions, next_checks=ordered[:3], catalogue_hypotheses=[dict(id=h["id"], status=h["status"]) for h in base["hypotheses"] if h["id"] != "unknown-cause"], applicable_passages=len(good), rejected_passages=len(rejected), rejected=[_cite(p) | dict(reason="conteúdo hostil" if p["hostile"] else "; ".join(p["applicability"]["notes"])) for p in rejected[:5]], confirmed_causes=[], commands_executed=False, execution_authorized=False, automatic_approval=False)


def _cite(p):
    return dict(third_party_instructions=bool(rd.INSTRUCTION.search(p["quote"])), url=p["url"], repo=p["repo"], thread=p["thread"], part_id=p["part_id"], kind=p["kind"], author=p["author"], association=p["association"], created_at=p["created_at"], updated_at=p["updated_at"], fetched_at=p["fetched_at"], sha256=p["sha256"], quote=p["quote"], nature=p["nature"], applicability=p["applicability"]["status"], notes=p["applicability"]["notes"], source=p["source"])


def report(state, strategy="iterative", policy="discriminate"):
    out = conclude(state, policy)
    origin = dict(rules="interpretação, roteamento, formulação e reformulação de consultas, escolha de pistas, avaliação, parada, hipóteses e verificações", learned="nenhum" if not getattr(state, "semantic", False) else "reordenação de resultados por MiniLM público congelado", ai_service="nenhum")
    return dict(version="frx-ai-v9", strategy=strategy, problem=state.facts["known"], unknown=state.facts["unknown"], queries=[{k: q.get(k) for k in ("terms", "kind", "reason", "repo", "lead", "total", "returned", "new_threads", "new_leads", "new_passages", "relaxed")} for q in state.queries], steps=state.steps, failures=state.failures, sources_examined=sorted({f"{r}#{n}" for r, n in state.seen}), origin=origin, **out)


def update_case(state, message=None, log=None, results=()):
    """A new turn: new facts change the state and what is asked. Prior evidence stays in memory."""
    new_terms = {}
    results = list(results)
    if message:
        message = rd.logs.redact(rd.sanitize(message))
        state.facts["text"] += "\n" + message
        new_terms.update(rd.terms_of(message))
        # "X exists": the absence reading loses support, with the person's words as the finding.
        for m in PRESENT.finditer(message):
            results.append(dict(check="inspect-import-chain", verdict="refutes", hypothesis="missing-windows-component", finding=m[0], source="user-reported-check"))
    if log:
        log = rd.logs.redact(rd.sanitize(log))
        state.facts["known"]["log_supplied"] = True
        state.facts["text"] += "\n" + log
        new_terms.update(rd.terms_of(log))
        state.facts["unknown"] = [u for u in state.facts["unknown"] if u != "log"]
    fresh = {t: k for t, k in new_terms.items() if t not in state.facts["input_terms"]}
    state.facts["input_terms"].update(new_terms)
    state.facts["symptoms"] = sorted({s["kind"] for s in rd.symptoms(state.facts["text"])})
    state.results.extend(results)
    # A new specific term becomes the most urgent lead: it is what the person just observed.
    for term, kind in fresh.items():
        if kind in ("library", "error", "option") and rd.safe_term(term):
            state.leads[term].update(kind=kind, score=state.leads[term]["score"] + 100.0)
            state.facts["input_terms"].pop(term, None)
    return fresh
