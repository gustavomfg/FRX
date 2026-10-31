"""A case state that grows with a conversation, and the choice of the next verification.

Every piece is an explicit rule. It keeps problem, evidence, hypotheses, verifications and
results apart, reports what is still unknown, and never confirms a cause.
"""
import copy
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "v7"))
sys.path.insert(0, str(HERE))
import investigate as v7  # noqa: E402
from knowledge import CHECKS as V7_CHECKS, HYPOTHESES  # noqa: E402
from signals import environment, log_signals, symptoms  # noqa: E402

import detector  # noqa: E402
import joint  # noqa: E402
import logs  # noqa: E402

ORIGIN = "rule"
EXTRA_CHECKS = {
    "read-attached-log": dict(what="Ler o log que a pessoa já anexou.", how="Abrir o anexo citado e procurar o erro. Este protótipo não baixa anexos.", cost=1, changes_state=False, plays_game=False, demo=None, provides=None),
    "bisect-proton-versions": dict(what="Encontrar a última versão que funcionou e a primeira que falhou.", how="Testar versões intermediárias, uma por vez, no mesmo trecho.", cost=4, changes_state=True, plays_game=True, demo="test-other-proton", provides=None),
}
CHECKS = {**V7_CHECKS, **EXTRA_CHECKS}
ATTACHED = re.compile(r"(?i)steam-\d+[\w-]*\.(?:log|zip|tar|gz)|proton\w*log\S*\.(?:zip|log)|\[[^\]]*\.log\]\(")
REGRESSION = re.compile(r"(?i)\b(?:regress\w*|used\s+to\s+work|was\s+working|last\s+(?:worked|rendered|working)|worked\s+(?:before|on|in|with)|stopped\s+working|after\s+(?:the\s+)?update)\b")
DONE = {
    "test-other-proton": re.compile(r"(?i)\b(?:tried|tested|test|switched|using|used|with)\b[^.\n]{0,50}\b(?:proton\s*(?:experimental|hotfix|\d)|GE-Proton|experimental|hotfix)|\bProton\s+Versions?\s+Tested\b"),
    "reset-prefix-with-backup": re.compile(r"(?i)\b(?:clean(?:ed|ing)?|fresh|new|delet\w*|clear\w*|remov\w*|recreat\w*|renam\w*|wip\w*)\b[^.\n]{0,40}\b(?:prefix|pfx|compatdata)\b"),
    "verify-game-files": re.compile(r"(?i)\bverif\w*\b[^.\n]{0,40}\b(?:files|integrity|game)\b|\breinstall\w*\b"),
    "review-launch-options": re.compile(r"(?i)\b(?:without|removed?|no)\b[^.\n]{0,20}\b(?:launch\s+options?|env\w*\s+var\w*)\b"),
    "check-driver-stack": re.compile(r"(?i)\b(?:driver\s+version|mesa\s+\d|nvidia\s+\d{3}|\d{3}\.\d{2,3}\.\d+)\b"),
    "collect-system-info": re.compile(r"(?i)system\s+information|gist\.github\.com|\b(?:GPU|Kernel|Processor)\s*(?:version)?\s*:"),
    "isolate-scope": re.compile(r"(?i)\b(?:other|another|all)\s+games?\b|\bon\s+windows\b"),
}
CHECK_OF_ACTION = [
    (re.compile(r"(?i)proton|experimental|hotfix"), "test-other-proton"),
    (re.compile(r"(?i)prefix|pfx|compatdata"), "reset-prefix-with-backup"),
    (re.compile(r"(?i)verif|reinstall"), "verify-game-files"),
    (re.compile(r"(?i)%command%|launch\s+option|\b[A-Z][A-Z0-9_]{3,}="), "review-launch-options"),
    (re.compile(r"(?i)driver|mesa|nvidia"), "check-driver-stack"),
]
BASELINE = ("collect-proton-log", "collect-system-info", "isolate-scope", "test-other-proton")
VERDICT = dict(resolved="supports", improved="supports", worsened="supports", not_resolved="refutes", mixed="inconclusive")


def unquoted(text):
    return re.sub(r"(?m)^\s*>.*$", "", re.sub(r"<!--.*?-->", "", text or "", flags=re.S))


def case_from_thread(title, body, comments, context=None):
    """Only what was already said. Comments are dicts with role and text, in order."""
    turns = [dict(role="reporter", text=unquoted(body))] + [dict(role=c["role"], text=unquoted(c["text"])) for c in comments]
    return dict(game=title, problem=turns[0]["text"], turns=turns, context=context or {})


def extract_results(turns):
    """Result claims stated by the author of each turn. Maintainer requests are not results."""
    out = []
    for n, turn in enumerate(turns):
        if turn["role"] == "maintainer":
            continue
        found = detector.detect(turn["text"])
        for proc in joint.procedures(turn["text"], found):
            checks = sorted({check for item in proc["interventions"] for rx, check in CHECK_OF_ACTION if rx.search(item["text"])})
            out.append(dict(turn=n, cls=proc["result"]["cls"], quote=proc["result"]["quote"], relation=proc["relation"], actions=[i["text"] for i in proc["interventions"]], checks=checks, attribution=proc["attribution"], origin=ORIGIN))
    return out


def state_of(case):
    """Problem, evidence, hypotheses and results kept separate, with the origin of each."""
    case = copy.deepcopy(case)
    turns = case.get("turns") or [dict(role="reporter", text=case.get("problem", ""))]
    said = "\n".join(t["text"] for t in turns if t["role"] != "maintainer")
    asked = "\n".join(t["text"] for t in turns if t["role"] == "maintainer")
    log = logs.redact(case.get("log") or "")
    clean_said = logs.redact(said)
    sym = symptoms(clean_said)
    sig = log_signals(log) + log_signals(clean_said, "supplied-problem-text-log-line")
    env = environment(clean_said + "\n" + log)
    hyps = v7.hypotheses(dict(case, problem=clean_said), sig, sym, env)
    results = extract_results(turns)
    applied = []
    for r in results:
        verdict = VERDICT.get(r["cls"])
        # A group's result reaches each member as inconclusive: it never credits one action.
        if r["relation"] in ("joint", "sequence", "unclear") and verdict != "refutes":
            verdict = "inconclusive"
        for check in r["checks"]:
            applied.append(dict(check=check, verdict=verdict, finding=r["quote"], source="reported-in-thread"))
    known = [dict(c) for c in case.get("results", [])]
    hyps = v7.apply_results(hyps, applied + known)
    done = {c for c, rx in DONE.items() if rx.search(clean_said)} | {x["check"] for x in applied + known}
    have = set()
    log_present = bool(log) or any(s["source"] == "supplied-log" for s in sig) or any(s["source"] != "supplied-problem-text" for s in sig)
    attached = bool(ATTACHED.search(said))
    if log_present or attached:
        have.add("log")
    if "collect-system-info" in done:
        have.add("info")
    return dict(game=case.get("game"), problem=turns[0]["text"][:600], symptoms=sym, signals=sig, environment=env, hypotheses=hyps, results=results, done=sorted(done), have=sorted(have), attached_log=attached, log_read=bool(log), regression=bool(REGRESSION.search(said)), asked=asked,
                reply_expected=bool(turns[-1]["role"] == "maintainer"), confirmed_causes=[], origin=ORIGIN)


def available(state):
    return {cid: c for cid, c in CHECKS.items() if cid not in state["done"] and not (c.get("provides") and c["provides"] in state["have"])}


def discriminate(state):
    """Prefer the verification that splits the open hypotheses, reading before changing things."""
    open_h = [h for h in state["hypotheses"] if h["status"] != "weakened-by-reported-check" and h["id"] != "unknown-cause"]
    technical = [h for h in open_h if any(s["source"] not in ("supplied-problem-text",) for s in h["supports"])]
    pool = technical or open_h
    scored = {}
    for cid, c in available(state).items():
        reach = [h for h in pool if cid in HYPOTHESES[h["id"]]["checks"]]
        split = min(len(reach), len(pool) - len(reach)) if len(pool) > 1 else len(reach)
        order = min((HYPOTHESES[h["id"]]["checks"].index(cid) for h in reach), default=9)
        score = (2.0 * split + (1.0 if reach else 0.0) + 1.0 * len([h for h in reach if h in technical]) - .25 * order
                 - .3 * c["cost"] - (1.0 if c.get("changes_state") else 0.0) - (.5 if c.get("plays_game") else 0.0))
        # A check that separates nothing open is not a reason to ask, except for basic collection.
        if not reach and not (not pool and cid in BASELINE):
            continue
        scored[cid] = (score, [h["id"] for h in reach])
    # Evidence the person already gave comes first; then evidence that is still missing.
    if state["attached_log"] and not state["log_read"] and not state["signals"]:
        scored["read-attached-log"] = (20.0, [h["id"] for h in pool])
    elif "log" not in state["have"] and "collect-proton-log" in scored:
        scored["collect-proton-log"] = (10.0, [h["id"] for h in pool])
    if state["regression"] and "bisect-proton-versions" in scored:
        scored["bisect-proton-versions"] = (scored["bisect-proton-versions"][0] + 6.0, scored["bisect-proton-versions"][1])
    else:
        scored.pop("bisect-proton-versions", None)
    if not state["attached_log"] or state["log_read"]:
        scored.pop("read-attached-log", None)
    order = sorted(scored, key=lambda k: (-scored[k][0], CHECKS[k]["cost"], k))
    return [dict(id=k, score=round(scored[k][0], 2), separates=scored[k][1], question=CHECKS[k]["what"], how=CHECKS[k]["how"], changes_state=bool(CHECKS[k].get("changes_state")), origin=ORIGIN) for k in order]


def always_log(state):
    order = ["collect-proton-log", "collect-system-info", "collect-terminal-output"]
    return [dict(id=k, origin=ORIGIN, changes_state=False) for k in order]


def rules_v7(state):
    """The v7 ranking on the same text, unaware of the conversation structure."""
    case = dict(game=state["game"], problem=state["problem"] + "\n" + "\n".join(s["quote"] for s in state["signals"]), context={}, results=[])
    return [dict(id=c["id"], origin=ORIGIN, changes_state=c["changes_state"]) for c in v7.analyse(case, "rules")["next_checks"]]


POLICIES = dict(discriminate=discriminate, always_log=always_log, rules_v7=rules_v7)


def report(case, policy="discriminate"):
    state = state_of(case)
    checks = POLICIES[policy](state)
    unknown = []
    if not state["signals"]:
        unknown.append("Nenhum sinal técnico lido até aqui.")
    if state["attached_log"] and not state["log_read"]:
        unknown.append("Existe log anexado, mas este protótipo não baixa anexos.")
    unknown.append("A causa continua não confirmada.")
    return dict(status="experimental-frx-v8", problem=state["problem"], evidence=dict(symptoms=state["symptoms"], signals=state["signals"]),
                hypotheses=[dict(id=h["id"], status=h["status"], supports=len(h["supports"]), against=len(h["contradictions"]), origin=h.get("origin", ORIGIN)) for h in state["hypotheses"]],
                verifications_done=state["done"], results=state["results"], next_checks=checks[:5], unknown=unknown,
                confirmed_causes=[], commands_executed=False, execution_authorized=False, automatic_approval=False,
                capability_origin=dict(rules="detector, grupos, hipóteses, escolha de verificação, sinais de log", retrieval="nenhuma nesta saída", learned="nenhuma"))
