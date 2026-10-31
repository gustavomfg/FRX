"""One investigation step from a case, and the loop across steps.

Nothing runs here: no command, no file change, no download. Each output says where
it came from, and hypotheses never become confirmed causes.
"""
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from knowledge import CHECKS, HYPOTHESES  # noqa: E402
from signals import environment, log_signals, symptoms  # noqa: E402

CONTEXT_KEYS = ["app_id", "gpu_vendor", "gpu", "driver", "proton", "game_version", "configuration"]
NEEDED = {"gpu_vendor": "fabricante da GPU", "driver": "versão do driver", "proton": "versão do Proton"}


def context_relation(a, b):
    """Same rule as v5 memory: unknown never matches, a difference never transfers."""
    if a.get("app_id") != b.get("app_id"):
        return "different-game"
    for k in CONTEXT_KEYS[1:]:
        if a.get(k) is not None and b.get(k) is not None and a[k] != b[k]:
            return "incompatible"
    if any(a.get(k) is None or b.get(k) is None for k in ["gpu_vendor", "driver", "proton"]):
        return "unknown"
    return "compatible"


class CasePrior:
    """Maintainer-demonstration prior: k nearest reports in the D7 training split."""
    def __init__(self, root):
        import selector
        rows = [json.loads(x) for x in (Path(root) / "d7.jsonl").read_text().splitlines()]
        self.model = selector.CaseRetrieval(15)
        self.model.fit([r for r in rows if r["split"] == "train"])
        self.demo = selector.CHECKS

    def scores(self, text):
        s = self.model.scores([dict(query=text)])[0]
        return {c: float(v) for c, v in zip(self.demo, s)}


def provided(case, log_found):
    text = (case.get("problem") or "") + "\n" + (case.get("log") or "")
    have = set()
    if case.get("log") or log_found:
        have.add("log")
    ctx = case.get("context", {})
    if all(ctx.get(k) for k in ["gpu_vendor", "driver", "proton"]):
        have.add("info")
    return have, text


def hypotheses(case, sig, sym, env):
    ctx = case.get("context", {})
    vendor = (ctx.get("gpu_vendor") or "").lower()
    kinds = {s["kind"] for s in sig}
    feel = {s["kind"] for s in sym}
    options = env.get("options_mentioned") or ctx.get("launch_options")
    out = []
    for hid, h in HYPOTHESES.items():
        support = [dict(kind=s["kind"], quote=s["quote"], source=s["source"]) for s in sig if s["kind"] in h["signals"]]
        support += [dict(kind=s["kind"], quote=s["quote"], source=s["source"]) for s in sym if s["kind"] in h["symptoms"]]
        against = []
        if h.get("always"):
            pass
        elif not support:
            continue
        if h.get("needs_options") and not options:
            continue
        if h.get("needs_vendor") and not vendor:
            continue
        if h.get("requires_vendor") and vendor and vendor != h["requires_vendor"]:
            against.append(dict(kind="hardware", quote=f"GPU informada: {vendor}", source="user-context"))
        if h.get("excludes_vendor") and vendor == h["excludes_vendor"]:
            continue
        out.append(dict(id=hid, statement=h["statement"], status="open", supports=support, contradictions=against, confirmed=False, origin="explicit-rule"))
    # An explicit technical signal outranks a symptom-only guess; unknown stays last.
    out.sort(key=lambda x: (x["id"] == "unknown-cause", -sum(s["source"] != "supplied-problem-text" for s in x["supports"]), -len(x["supports"]), len(x["contradictions"])))
    return out


def apply_results(hyps, results):
    """A check result changes the state of a hypothesis, never into a cause."""
    for r in results:
        for h in hyps:
            if h["id"] != r.get("hypothesis") and r["check"] not in HYPOTHESES.get(h["id"], {}).get("checks", []):
                continue
            if r.get("hypothesis") and h["id"] != r["hypothesis"]:
                continue
            item = dict(kind="check-result", check=r["check"], quote=r.get("finding", ""), source="user-reported-check")
            if r["verdict"] == "supports":
                h["supports"].append(item)
                h["status"] = "supported-by-reported-check"
            elif r["verdict"] == "refutes":
                h["contradictions"].append(item)
                h["status"] = "weakened-by-reported-check"
            else:
                h.setdefault("inconclusive", []).append(item)
    return hyps


def rank_checks(case, hyps, have, done, prior, policy):
    """Which verification next, and exactly why. Policy is explicit for evaluation."""
    open_h = [h for h in hyps if h["status"] != "weakened-by-reported-check"]
    technical = [h for h in open_h if any(s["source"] not in ("supplied-problem-text",) for s in h["supports"])]
    scored = {}
    for cid, c in CHECKS.items():
        if cid in done or (c.get("provides") and c["provides"] in have):
            continue
        reach = [h for h in open_h if cid in HYPOTHESES[h["id"]]["checks"]]
        strong = [h for h in technical if cid in HYPOTHESES[h["id"]]["checks"]]
        position = min((HYPOTHESES[h["id"]]["checks"].index(cid) for h in reach), default=9)
        # Leading hypotheses count more, so a specific reading beats a broad one.
        weight = sum(1.0 / (1 + open_h.index(h)) for h in reach)
        rule = 3.0 * len(strong) + weight - 0.35 * c["cost"] - 0.25 * position - (1.0 if c.get("changes_state") else 0.0) - (0.5 if c.get("plays_game") else 0.0)
        # A symbol error that names no file: who asked for it must be known first.
        if cid == "identify-requiring-object" and not any(".so" in s["quote"] for h in strong for s in h["supports"] if s["kind"] == "undefined-symbol"):
            rule += 5.0
        top = max(prior.values(), default=0.0) or 1.0
        demo = prior.get(c.get("demo"), 0.0) / top if c.get("demo") else 0.0
        if policy == "rules":
            score = rule if reach else -5.0
        elif policy == "case-prior":
            score = demo if c.get("demo") else -5.0
        else:
            # Hybrid: typed signals and specific hypotheses decide; demonstrations break ties.
            score = rule + 0.5 * demo if reach or c.get("demo") else -5.0
        reasons = []
        if reach:
            reasons.append("distingue: " + ", ".join(h["id"] for h in reach[:3]))
        if demo and policy != "rules":
            reasons.append("mantenedores pediram isto em relatos parecidos")
        if c.get("changes_state"):
            reasons.append("altera o estado: fica depois das leituras")
        scored[cid] = (score, reasons, [h["id"] for h in reach])
    order = sorted(scored, key=lambda k: (-scored[k][0], CHECKS[k]["cost"], k))
    out = []
    for cid in order:
        if scored[cid][0] <= -5.0:
            continue
        c = CHECKS[cid]
        out.append(dict(id=cid, question=c["what"], how=c["how"], why="; ".join(scored[cid][1]) or "coleta de base", discriminates=scored[cid][2], score=round(scored[cid][0], 3), plays_game=bool(c.get("plays_game")), changes_state=bool(c.get("changes_state")), read_only=not c.get("changes_state"), origin={"rules": "explicit-rule", "case-prior": "case-retrieval-prior", "hybrid": "hybrid-rule-and-case-prior"}[policy], executed=False))
    return out


def analyse(case, policy="hybrid", prior_model=None, evidence=None, reader=None):
    if not isinstance(case.get("problem"), str) or not case["problem"].strip():
        raise ValueError("descrição do problema é obrigatória")
    case = copy.deepcopy(case)
    ctx = case.setdefault("context", {})
    log = case.get("log") or ""
    text = (case.get("game") or "") + "\n" + case["problem"]
    sym = symptoms(case["problem"])
    # A log pasted inside the problem counts as supplied text, with its own source.
    sig = log_signals(log) + log_signals(case["problem"], "supplied-problem-text-log-line")
    env = environment(case["problem"] + "\n" + log)
    hyps = hypotheses(case, sig, sym, env)
    results = case.get("results", [])
    done = {r["check"] for r in results} | set(case.get("checked_items", []))
    hyps = apply_results(hyps, results)
    have, _ = provided(case, [s for s in sig if s["source"] == "supplied-log"])
    if any(s["source"] != "supplied-problem-text" for s in sig):
        have.add("log")
    # A linked log exists but is not read: asking again wastes the person's time,
    # and reading it first becomes the honest next step for them.
    attached = bool(re.search(r"(?i)steam-\d+[\w-]*\.(?:log|zip|tar)|proton\w*log\S*\.(?:zip|log)", case["problem"]))
    if attached:
        have.add("log")
    prior = prior_model.scores(text + "\n" + log[:1500]) if prior_model and policy != "rules" else {}
    checks = rank_checks(case, hyps, have, done, prior, policy)
    missing = [dict(field=k, label=v) for k, v in NEEDED.items() if not ctx.get(k)]
    if not ctx.get("app_id"):
        missing.append(dict(field="app_id", label="AppID do jogo"))
    if not log and "log" not in have:
        missing.append(dict(field="log", label="log do Proton da falha"))
    found = []
    if evidence is not None:
        query = " ".join(filter(None, [case.get("game"), case["problem"][:500], " ".join(s["quote"][:160] for s in sig[:3])]))
        found = evidence.search(query, case)
        for f in found:
            f["reported_procedures"] = reader(f) if reader else []
    understanding = dict(symptoms=[s["kind"] for s in sym], technical_signals=[s["kind"] for s in sig if s["source"] != "supplied-problem-text"], ambiguous=len({s["kind"] for s in sym}) > 2 or not sym)
    uncertainty = []
    if not sym and not sig:
        uncertainty.append("Nem o texto nem o log trazem um sintoma reconhecido.")
    if not any(s["source"] != "supplied-problem-text" for s in sig):
        uncertainty.append("Nenhum sinal técnico no material fornecido: as hipóteses vêm só da descrição.")
    if env.get("gpu_vendor_mentions") and ctx.get("gpu_vendor") and ctx["gpu_vendor"].lower() not in env["gpu_vendor_mentions"]:
        uncertainty.append("O texto cita uma GPU diferente da informada no contexto.")
    if attached and not log:
        uncertainty.append("O texto cita um log anexado, mas este protótipo não o baixa nem o lê: forneça o arquivo com --log.")
    uncertainty.append("Relatos encontrados são associação relatada, não causa nem resultado neste computador.")
    return dict(
        status="experimental-frx-v7-investigation", game=case.get("game"), context=ctx, understanding=understanding,
        symptoms=sym, signals=sig, environment_mentions=env, missing_information=missing, hypotheses=hyps,
        confirmed_causes=[], cause_probability=None, next_check=checks[0] if checks else None, next_checks=checks[:5],
        history=dict(results=results, checked_items=sorted(done)), evidence=found, uncertainty=uncertainty,
        policy=policy, provenance=dict(hypotheses="regras explícitas", next_checks=checks[0]["origin"] if checks else None, evidence="corpus local público; sem rede" if found else "sem recuperação nesta execução", learned_diagnostic_model=False, neural="somente o reranker FRX v2 da recuperação" if found else "nenhum"),
        online_search_performed=False, commands_executed=False, execution_authorized=False, automatic_approval=False,
    )


def load_case(path):
    return json.loads(Path(path).read_text()) if path and Path(path).exists() else None


def merge_history(new, stored):
    """History persists only for the same environment; otherwise it is kept apart."""
    if not stored:
        return new, []
    relation = context_relation(new.get("context", {}), stored.get("context", {}))
    if relation in ("different-game", "incompatible"):
        return new, [dict(relation=relation, ignored_results=len(stored.get("results", [])), reason="ambiente diferente: resultados anteriores não são transferidos")]
    merged = dict(stored, **{k: v for k, v in new.items() if v not in (None, "", [], {})})
    merged["results"] = stored.get("results", []) + new.get("results", [])
    merged["context"] = dict(stored.get("context", {}), **{k: v for k, v in new.get("context", {}).items() if v})
    merged["log"] = new.get("log") or stored.get("log")
    return merged, []


def digest(case):
    return hashlib.sha256(json.dumps(case, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


__all__ = ["analyse", "CasePrior", "load_case", "merge_history", "context_relation"]
