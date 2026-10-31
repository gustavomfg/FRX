#!/usr/bin/env python3
"""FRX AI v9 laboratory CLI. Reads public sources and changes nothing on this computer.

Online by default on the public GitHub API, without credentials. Offline uses the frozen snapshot.
Each printed step is an operation that really happened. Logs come only from v8 fixtures.
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import engine  # noqa: E402
import net  # noqa: E402
import sources  # noqa: E402
import reading as rd  # noqa: E402

LABEL = dict(understand="Entendendo o problema", search="Pesquisando", reformulate="Reformulando a consulta", read="Lendo", assess="Avaliando o que foi lido")


def stepper(trace, stream):
    count = [0]

    def say(kind, data):
        count[0] += 1
        if kind == "search" or kind == "reformulate":
            detail = f"{data['repo']}: {' '.join(data['terms'])} [{data['kind']}] — {data['reason']}"
        elif kind == "read":
            detail = f"{data['title']} ({data['url']})"
        elif kind == "assess":
            detail = f"{data['new_threads']} threads novos; pistas novas: {', '.join(data['new_leads']) or 'nenhuma'}"
        else:
            detail = "conhecido: " + ", ".join(f"{k}={v}" for k, v in data["known"].items()) + " | desconhecido: " + ", ".join(data["unknown"])
        if trace or kind != "read":
            print(f"[{count[0]}] {LABEL[kind]}: {rd.sanitize(detail)[:220]}", file=stream, flush=True)
    return say


def render(r, client):
    p = r["problem"]
    print("\n## Problema")
    print(f"{p.get('game') or 'Jogo não informado'} — {p.get('symptom') or 'sintoma não informado'}")
    print(f"GPU: {p.get('gpu') or 'não informada'} | Driver: {p.get('driver') or 'não informado'} | Proton: {p.get('proton') or 'não informado'}")
    print(f"\n## Estado: {'evidência insuficiente' if r['status'] == 'insufficient-evidence' else 'conclusão provisória, nada confirmado'}")
    print(f"Fontes examinadas: {len(r['sources_examined'])}; trechos aplicáveis: {r['applicable_passages']}; trechos rejeitados: {r['rejected_passages']}")
    print("\n## Pistas e hipóteses (relatos, não causas)")
    if not r["findings"]:
        print("Nenhuma pista aplicável encontrada.")
    for f in r["findings"]:
        print(f"- {f['term']} [{f['kind']}]: {f['status']}; {f['threads']} thread(s); a favor {f['positive']}, contra {f['negative']}")
        for e in f["evidence"][:1]:
            print(f"    “{e['quote'][:200]}”\n    {e['url']} | {e['nature']} | {e['applicability']} | {(e['created_at'] or '')[:10]}")
            if e["third_party_instructions"]:
                print("    Atenção: citação de terceiros com instruções. Não é recomendação e não foi verificada.")
            for n in e["notes"]:
                print(f"    Limite: {n}")
    print("\n## Contradições")
    print("\n".join(f"- {f['term']}: a favor {f['positive']}, contra {f['negative']}" for f in r["contradictions"]) or "Nenhuma identificada nas fontes lidas.")
    print("\n## Pesquisas realizadas")
    for q in r["queries"]:
        print(f"- {q['repo']}: {' '.join(q['terms'])} [{q['kind']}] — {q['reason']}; {q.get('returned', 0)} resultados, {q.get('new_threads', 0)} threads novos, {len(q.get('new_leads') or [])} pistas novas")
    print("\n## Próxima verificação (do catálogo próprio; nada foi executado)")
    for c in r["next_checks"]:
        print(f"- {c['id']}: {c['question']} Motivo: {c['why']}.\n    Como: {c['how']}")
    if r["rejected"]:
        print("\n## Evidência rejeitada")
        for e in r["rejected"]:
            print(f"- {e['url']}: {e['reason']}")
    print("\n## Limitações")
    print("- Nenhuma conclusão foi verificada neste computador e nenhuma correção foi aplicada.")
    print("- Um relato de \"funcionou\" não é correção comprovada, e um thread fechado não significa que o problema foi resolvido.")
    for x in r["failures"]:
        print(f"- Falha: {x['action']}: {x['reason']}")
    print("\n## Origem")
    print(f"Regras: {r['origin']['rules']}.\nModelo aprendido: {r['origin']['learned']}. Serviço de IA: {r['origin']['ai_service']}.")
    if client:
        print(f"Rede: {client.network_requests} requisições; {len(client.rejections)} rejeitadas.")


def ask(label):
    try:
        return input(f"{label}: ").strip() or None
    except EOFError:
        return None


if __name__ == "__main__":
    a = argparse.ArgumentParser(description=__doc__)
    a.add_argument("--game")
    a.add_argument("--app-id")
    a.add_argument("--gpu")
    a.add_argument("--driver")
    a.add_argument("--proton")
    a.add_argument("--problem")
    a.add_argument("--fixture", help="log de v8/fixtures; outros arquivos são recusados")
    a.add_argument("--offline", action="store_true", help="usa só o snapshot congelado, sem rede")
    a.add_argument("--strategy", choices=["local", "single", "single-broad", "fixed", "random", "iterative"], default="iterative")
    a.add_argument("--until", help="ignora fontes posteriores a esta data (AAAA-MM-DD)")
    a.add_argument("--trace", action="store_true")
    a.add_argument("--json", action="store_true")
    o = a.parse_args()
    problem = dict(game=o.game, app_id=o.app_id, gpu=o.gpu, driver=o.driver, proton=o.proton, symptom=o.problem)
    if not (o.game or o.problem):
        print("FRX AI v9 — Autonomous Technical Research (experimental; somente leitura)\n")
        problem = dict(game=ask("Jogo"), app_id=ask("AppID"), gpu=ask("GPU"), driver=ask("Driver"), proton=ask("Proton"), symptom=ask("Problema"))
    if o.fixture:
        try:
            problem["log"] = rd.logs.read_fixture(o.fixture)
        except PermissionError as e:
            raise SystemExit(str(e))
    client = None
    if o.offline:
        tool = sources.Frozen()
    else:
        client = net.Client(max_requests=25)
        tool = sources.GitHub(client)
    state = engine.investigate(problem, tool, o.strategy, until=o.until, on_step=None if o.json else stepper(o.trace, sys.stdout))
    result = engine.report(state, o.strategy)
    if o.json:
        result["network"] = dict(requests=client.network_requests, calls=client.calls, rejections=client.rejections) if client else dict(mode="frozen-snapshot")
        print(json.dumps(result, indent=1, ensure_ascii=False, default=str))
    else:
        render(result, client)
