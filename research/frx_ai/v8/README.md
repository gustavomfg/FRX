# FRX AI v8 — detecção de resultados, procedimentos e trajetórias

Laboratório experimental. **Não diagnostica de forma autônoma, não aplica correções e não está no aplicativo.** Não treina nada: tudo o
que acrescenta é regra. Os rótulos são H0, do agente, e **não são validação humana**.

Resultados: [REPORT.md](REPORT.md). Regras fixadas antes dos dados: [PROTOCOL.md](PROTOCOL.md).
Benchmark humano, separado: [`../benchmark/`](../benchmark/PILOT-PROTOCOL.md).

| Peça | Arquivo | Origem |
|---|---|---|
| Detector de resultados | `detector.py` | Regra |
| Procedimentos e grupos | `joint.py` | Regra |
| Estado do caso e próxima verificação | `investigator.py` | Regra |
| Trajetórias e replay | `trajectory.py` | Medição |
| Logs, redação e bloqueio de caminhos | `logs.py`, `fixtures/` | Regra; fixtures inventadas |
| Simulador de lógica | `simulate.py` | Teste circular |
| Comparadores congelados | `compare.py` | v7, regex do v2 e pesos E07, sem treino |

## Testar no Fish

Na raiz do repositório. A CLI só usa a biblioteca padrão.

```fish
set LAB research/frx_ai/v8/frx_investigate.py
python3 $LAB --game "Jogo" --problem "The game crashes on start. I tried Proton 9 and Experimental, same crash." --fixture missing-dll.log
```

Com o resultado de uma verificação:

```fish
python3 $LAB --problem "The game crashes" --fixture page-fault.log --result "locate-faulting-module=supports:falha dentro de d3d11"
```

Fixtures disponíveis: arquivos `.log` em `research/frx_ai/v8/fixtures/`. **São inventadas e testam as regras.**

**A CLI recusa qualquer log fora de `fixtures/`.** Ler arquivos privados, da Steam ou de jogos exige nova autorização específica.

Cada linha traz a origem. `--json` entrega a saída estruturada, e `--policy` compara políticas.

## Reproduzir

```fish
set FRX_PY $HOME/.cache/frx-ai-rnd-v6-20261030/venv/bin/python
python3 research/frx_ai/v8/freeze.py --verify
python3 -m unittest discover -s research/frx_ai/v8 -p 'test_contracts.py'
python3 research/frx_ai/v8/simulate.py
python3 research/frx_ai/v8/posthoc.py
```

Corpos e rótulos completos ficam em `~/.cache/frx-ai-v8-20261030`, e `snapshot/` traz URLs, hashes, rótulos sem texto e métricas.
`evaluate.py first` recusa rodar de novo: **repetir nos mesmos dados é replay, não teste.** Uma nova leitura precisa de código
congelado, coorte nova e rótulos travados antes das previsões.

## O que não funciona

- Afirmações de resultado: 22 de 32 achadas em dados novos. A linguagem real foge da lista de regras.
- Procedimentos conjuntos: 1 de 14 grupos.
- Acompanhar uma conversa: 7 de 18 resultados, com muito ruído.
- A política recomenda ler anexos, mas o protótipo não lê anexos.
- Sinais de log: 3 de 7 trechos relevantes, contra 6 de 7 de uma busca literal.
- Português não foi avaliado, e nada foi verificado em um jogo real.
