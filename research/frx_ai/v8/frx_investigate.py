#!/usr/bin/env python3
"""FRX AI v8 laboratory CLI. Reads a description and fixture logs; executes and changes nothing.

Each line says its origin: [você], [regra], [recuperação]. Nothing here is learned.
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import investigator as inv  # noqa: E402
import logs  # noqa: E402

VERDICTS = ("supports", "refutes", "inconclusive")


def parse_result(text):
    check, _, rest = text.partition("=")
    verdict, _, finding = rest.partition(":")
    if check not in inv.CHECKS or verdict not in VERDICTS:
        raise SystemExit(f"resultado inválido: use verificação=supports|refutes|inconclusive:texto ({text})")
    return dict(check=check, verdict=verdict, finding=finding, source="user-reported-check")


def render(out, log_reading):
    print("Laboratório FRX AI v8. Não aplica correções e não confirma causas.\n")
    print(f"[você] Problema: {out['problem'].strip()[:300]}")
    for s in out["evidence"]["symptoms"]:
        print(f"[regra] Sintoma ({s['kind']}): “{s['quote'][:100]}”")
    if log_reading:
        for s in log_reading["signals"]:
            print(f"[regra] Sinal do log ({s['kind']}): “{s['quote'][:140]}”")
        for x in log_reading["limits"]:
            print(f"[regra] Limite: {x}")
    if out["results"]:
        print()
        for r in out["results"]:
            members = "; ".join(r["actions"][:4]) or "ação não identificada"
            print(f"[regra] Resultado relatado ({r['cls']}, {r['relation']}): {members} → “{r['quote']}”")
    print("\nHipóteses abertas, nenhuma confirmada:")
    for h in out["hypotheses"][:6]:
        print(f"[regra] {h['id']}: {h['status']} (a favor {h['supports']}, contra {h['against']})")
    print("\nPróximas verificações:")
    for c in out["next_checks"][:3]:
        separates = ", ".join(c.get("separates", [])[:3]) or "coleta de base"
        print(f"[regra] {c['id']}: {c.get('question', '')} Separa: {separates}.")
        if c.get("how"):
            print(f"        Como: {c['how']}")
    print("\nAinda desconhecido:")
    for x in out["unknown"]:
        print(f"- {x}")
    origin = out["capability_origin"]
    print(f"\nOrigem: regras = {origin['rules']}; recuperação = {origin['retrieval']}; aprendido = {origin['learned']}.")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--problem", required=True)
    p.add_argument("--game", default="")
    p.add_argument("--fixture", help="nome de um arquivo em fixtures/; outros arquivos são recusados")
    p.add_argument("--result", action="append", default=[], help="verificação=supports|refutes|inconclusive:texto")
    p.add_argument("--policy", choices=list(inv.POLICIES), default="discriminate")
    p.add_argument("--json", action="store_true")
    a = p.parse_args()
    log = None
    if a.fixture:
        try:
            log = logs.read_fixture(a.fixture)
        except PermissionError as e:
            raise SystemExit(str(e))
    case = dict(game=a.game, problem=a.problem, turns=[dict(role="reporter", text=a.problem)], log=log, results=[parse_result(x) for x in a.result], context={})
    out = inv.report(case, a.policy)
    reading = logs.read_log(log) if log else None
    if a.json:
        print(json.dumps(dict(out, log_reading=reading), indent=1, ensure_ascii=False))
    else:
        render(out, reading)
