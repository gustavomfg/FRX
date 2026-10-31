#!/usr/bin/env python3
"""FRX AI v7 lab CLI. Reads what you give it and suggests the next verification.

Runs no command, changes no file outside --case-file, and has no network access.
"""
import argparse
import json
import os
import sys
import textwrap
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "v6")]
CACHE = Path(os.environ.get("FRX_V7_CACHE", Path.home() / ".cache/frx-ai-rnd-v7-20261030"))
OLD = Path(os.environ.get("FRX_V2_CACHE", Path.home() / ".cache/frx-ai-rnd-v2-20261029"))
TAGS = {"supplied-problem-text": "você", "supplied-problem-text-log-line": "você/log", "supplied-log": "seu log", "user-context": "você", "user-reported-check": "sua verificação", "explicit-rule": "regra", "case-retrieval-prior": "casos", "hybrid-rule-and-case-prior": "regra+casos"}
MODALITY = {"improved": "melhorou", "persistent": "não resolveu", "regression": "piorou"}
ATTRIBUTION = {"single-reported": "uma ação relatada", "joint-unattributed": "conjunto; contribuição individual desconhecida", "distributive-attempts": "tentativas separadas, um só relato"}


def ask(label, default=None):
    try:
        got = input(f"{label}{' [' + default + ']' if default else ''}: ").strip()
    except EOFError:
        return default
    return got or default


def wrap(text, indent="    "):
    return textwrap.fill(str(text), width=96, initial_indent=indent, subsequent_indent=indent)


def build_reader(root):
    """Procedure reading of each retrieved paragraph, with structure kept."""
    from bridge import CompactSyntax
    import events
    import procedures
    inherited = events.evidence
    events.evidence = lambda text, a, b: inherited(text, min(a, b), max(a, b))
    parser = CompactSyntax(root)
    catalog = json.loads((root / "catalog.json").read_text())

    def read(item):
        row = dict(text=item["text"][:1300], app_id=item.get("app_id"), url=item["url"], document_id=item["url"], reporter_id=None)
        try:
            pred = procedures.analyze(row, parser, catalog, 3.4)
        except (ValueError, KeyError, IndexError):
            return []
        return [dict(result=c["evidence"]["text"], outcome=c["outcome"], actions=[pred["events"][i]["trigger"]["text"] for i in c["event_indices"]], structure=c["structure"], attribution=c["attribution"]["kind"], execution_supported=c["execution_supported"], causality_confirmed=False) for c in pred["observations"]]
    return read


def show(r):
    line = "─" * 72
    print(f"\n{line}\nFRX AI v7 · laboratório experimental · {r.get('game') or 'jogo não informado'}\n{line}")
    ctx = {k: v for k, v in r["context"].items() if v}
    print("\nEntendimento")
    print(wrap("Sintomas: " + (", ".join(r["understanding"]["symptoms"]) or "nenhum reconhecido") + "  [regra sobre seu texto]"))
    print(wrap("Sinais técnicos: " + (", ".join(r["understanding"]["technical_signals"]) or "nenhum no material fornecido") + "  [regra sobre seu log]"))
    if ctx:
        print(wrap("Ambiente informado: " + ", ".join(f"{k}={v}" for k, v in ctx.items()) + "  [você]"))
    if r["missing_information"]:
        print(wrap("Falta informar: " + ", ".join(m["label"] for m in r["missing_information"])))
    if r["signals"]:
        print("\nEvidência fornecida (trecho literal)")
        for s in r["signals"][:6]:
            print(wrap(f"{s['kind']}: {s['quote'][:170]}  [{TAGS.get(s['source'], s['source'])}]"))
    print("\nHipóteses (nenhuma é causa confirmada)")
    for h in r["hypotheses"][:5]:
        print(wrap(f"• {h['id']} — {h['statement']}  [regra]"))
        state = {"open": "em aberto", "supported-by-reported-check": "reforçada por verificação sua", "weakened-by-reported-check": "enfraquecida por verificação sua"}[h["status"]]
        print(wrap(f"estado: {state}; a favor: {len(h['supports'])}; contra: {len(h['contradictions'])}", "      "))
        for c in h["contradictions"][:2]:
            print(wrap(f"contra: {c['quote'][:120]}", "      "))
    n = r["next_check"]
    print("\nPróxima verificação recomendada")
    if n:
        print(wrap(f"{n['id']}: {n['question']}  [{TAGS.get(n['origin'], n['origin'])}]"))
        print(wrap("Como (você executa; o FRX não executa nada): " + n["how"], "      "))
        print(wrap("Por quê: " + n["why"], "      "))
        for o in r["next_checks"][1:4]:
            print(wrap(f"depois: {o['id']} — {o['question']}"))
    else:
        print(wrap("Nenhuma verificação pendente no catálogo; a evidência atual não fecha o caso."))
    if r["evidence"]:
        print("\nFontes parecidas (corpus local, sem rede; relevância não é aplicabilidade)")
        for e in r["evidence"][:4]:
            a = e["applicability"]
            print(wrap(f"• {e['title'][:60]} — {a['authority']}, {a['year']}, jogo: {a['game']}, GPU: {a['hardware']}  [{e['method']}]"))
            print(wrap(e["url"], "      "))
            print(wrap("“" + e["text"][:190].replace("\n", " ") + "”", "      "))
            for n2 in a["notes"]:
                print(wrap("atenção: " + n2, "      "))
            for p in e.get("reported_procedures", [])[:2]:
                print(wrap(f"relato: {', '.join(p['actions'])} → {MODALITY[p['outcome']]} ({ATTRIBUTION[p['attribution']]}; execução {'afirmada' if p['execution_supported'] else 'não afirmada'}) [extrator v7]", "      "))
    print("\nIncertezas")
    for u in r["uncertainty"]:
        print(wrap("• " + u))
    if r.get("history", {}).get("results"):
        print("\nHistórico deste caso")
        for x in r["history"]["results"]:
            print(wrap(f"• {x['check']}: {x['verdict']} — {x.get('finding', '')}  [{TAGS['user-reported-check']}]"))
    for x in r.get("context_rejections", []):
        print(wrap(f"Histórico ignorado: {x['reason']}"))
    p = r["provenance"]
    print(f"\nOrigem: hipóteses = {p['hypotheses']}; verificação = {p['next_checks']}; fontes = {p['evidence']}; neural = {p['neural']}; modelo de diagnóstico treinado = não")
    print("Pesquisa online: não. Comandos executados: nenhum. Correção autorizada: não.\n")


def main():
    p = argparse.ArgumentParser(description="FRX AI v7: investigação experimental, somente leitura.")
    p.add_argument("--game")
    p.add_argument("--app-id")
    p.add_argument("--problem")
    p.add_argument("--gpu-vendor", choices=["nvidia", "amd", "intel"])
    p.add_argument("--gpu")
    p.add_argument("--driver")
    p.add_argument("--proton")
    p.add_argument("--launch-options")
    p.add_argument("--log", type=Path, help="arquivo de log que você autoriza a ler")
    p.add_argument("--case-file", type=Path, help="guarda o histórico deste caso neste arquivo")
    p.add_argument("--result", action="append", default=[], metavar="CHECK=supports|refutes|inconclusive[:achado]", help="resultado de uma verificação que você fez")
    p.add_argument("--policy", choices=["rules", "hybrid", "case-prior"], default="rules", help="rules foi a melhor em C7; ver REPORT.md")
    p.add_argument("--no-sources", action="store_true", help="não consultar o corpus local")
    p.add_argument("--json", action="store_true", help="saída estruturada")
    a = p.parse_args()
    from investigate import CasePrior, analyse, load_case, merge_history
    if not a.problem and sys.stdin.isatty():
        print("FRX AI v7 · laboratório. Enter pula um campo.")
        a.game = a.game or ask("Jogo")
        a.app_id = a.app_id or ask("AppID")
        a.problem = ask("Problema")
        a.gpu_vendor = a.gpu_vendor or ask("Fabricante da GPU (nvidia/amd/intel)")
        a.driver = a.driver or ask("Driver")
        a.proton = a.proton or ask("Proton")
        path = ask("Arquivo de log")
        a.log = a.log or (Path(path).expanduser() if path else None)
    stored = load_case(a.case_file)
    if not a.problem and stored:
        a.problem = stored.get("problem")
    if not a.problem:
        p.error("informe --problem")
    log = None
    if a.log:
        if a.log.stat().st_size > 8_000_000:
            p.error("log maior que 8 MB")
        log = a.log.read_text(errors="replace")
    results = []
    for item in a.result:
        check, _, rest = item.partition("=")
        verdict, _, finding = rest.partition(":")
        if verdict not in ("supports", "refutes", "inconclusive"):
            p.error(f"veredito inválido em {item}")
        results.append(dict(check=check, verdict=verdict, finding=finding))
    case = dict(game=a.game, problem=a.problem, log=log, results=results, context={k: v for k, v in dict(app_id=a.app_id, gpu_vendor=a.gpu_vendor, gpu=a.gpu, driver=a.driver, proton=a.proton, launch_options=a.launch_options).items() if v})
    case, rejected = merge_history(case, stored)
    evidence, reader = None, None
    if not a.no_sources and (CACHE / "evidence-index").exists():
        from retrieve import Evidence
        from bridge import retriever
        evidence = Evidence(CACHE, retriever(OLD) if (OLD / "runtime-retrieval").exists() else None)
        reader = build_reader(CACHE)
    prior = CasePrior(CACHE) if a.policy != "rules" and (CACHE / "d7.jsonl").exists() else None
    result = analyse(case, a.policy, prior, evidence, reader)
    result["context_rejections"] = rejected
    if a.case_file:
        a.case_file.write_text(json.dumps({k: v for k, v in case.items() if k != "log" or v}, indent=1, ensure_ascii=False))
    if a.json:
        print(json.dumps(result, ensure_ascii=False, indent=1))
    else:
        show(result)


if __name__ == "__main__":
    main()
