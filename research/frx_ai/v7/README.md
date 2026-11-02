# FRX AI v7 — Diagnostic Reasoning & Candidate Discovery

Laboratório experimental. **Não é um agente que resolve problemas, nenhuma saída aprova correção, e
nada aqui entra no Fix Engine.** Ele não executa comandos, não baixa nada e não altera a Steam,
jogos, prefixos ou drivers. Os rótulos são provisórios do agente, sem revisão humana independente.

Resultados e erros: [REPORT.md](REPORT.md). Regras fixadas antes dos dados: [PROTOCOL.md](PROTOCOL.md).
Fontes: [LITERATURE.md](LITERATURE.md).

## O que foi desenvolvido

| Peça | Arquivo | Natureza |
|---|---|---|
| Atribuição de perdas por etapa | `waterfall.py` | Medição |
| Candidatos-oráculo (só diagnóstico) | `oracle.py` | Medição; a inferência não importa |
| Gerador de candidatos e estrutura de procedimentos | `procedures.py` | Regras sobre parser pré-treinado |
| Sinais de log e sintomas com citação literal | `signals.py` | Regras |
| Hipóteses e verificações | `knowledge.py` | Conhecimento escrito à mão |
| Ciclo de investigação com histórico | `investigate.py` | Regras |
| Busca local com proveniência | `retrieve.py` | TF-IDF mais recuperador neural FRX v2 preservado |
| Demonstrações de mantenedores e políticas | `demos.py`, `selector.py` | Modelos treinados só para comparação |
| Laboratório no terminal | `frx_lab.py` | CLI legível e JSON |

**Nenhum modelo de diagnóstico treinado é usado por padrão**, porque nenhum superou o baseline.

## Testar no Fish

Na raiz do repositório:

```fish
set -x FRX_V7_CACHE $HOME/.cache/frx-ai-rnd-v7-20261030
set -x FRX_V2_CACHE $HOME/.cache/frx-ai-rnd-v2-20261029
set FRX_PY $HOME/.cache/frx-ai-rnd-v6-20261030/venv/bin/python
set LAB research/frx_ai/v7/frx_lab.py
```

Modo perguntado (Enter pula um campo):

```fish
$FRX_PY $LAB
```

Um caso direto, com fontes do corpus local:

```fish
$FRX_PY $LAB --game "Batman: Arkham City GOTY" --app-id 200260 --gpu-vendor nvidia --problem "Desempenho ruim com PhysX"
```

Com um log seu, que só é lido:

```fish
$FRX_PY $LAB --game "Meu jogo" --app-id 10 --gpu-vendor amd --proton "GE-Proton10-4" --problem "The game does not start" --log $HOME/steam-10.log
```

Ciclo com histórico: rode, faça a verificação recomendada e informe o resultado.

```fish
$FRX_PY $LAB --app-id 10 --gpu-vendor nvidia --problem "The game crashes on start" --log $HOME/steam-10.log --case-file /tmp/caso-10.json
$FRX_PY $LAB --app-id 10 --gpu-vendor nvidia --case-file /tmp/caso-10.json --result "resolve-library-path=supports:carregou a cópia do runtime"
```

Vereditos: `supports`, `refutes` ou `inconclusive`. Se trocar `--gpu-vendor`, o histórico é ignorado,
e a saída avisa.

Outras opções: `--json` para automação, `--no-sources` para responder em cerca de 0,06 s e sem o índice,
`--policy rules|hybrid|case-prior` para comparar políticas.

Cada linha diz a origem: `[você]`, `[seu log]`, `[sua verificação]`, `[regra]`, `[casos]`,
`[tfidf+frx-v2-encoder]` ou `[extrator v7]`.

## Reproduzir

> **Congelado em 30/10/2026:** não rode `selector.py`, que treina em D7, até concluir a
> [auditoria de proveniência](../audit/PROVENANCE-AUDIT.md). As opções `--policy hybrid` e `case-prior` montam um
> índice sobre D7 e ficam restritas à comparação interna.

Corpos, índice e rótulos completos ficam em `~/.cache/frx-ai-rnd-v7-20261030`; `snapshot/` traz offsets,
URLs, hashes e métricas, sem corpos. Os scripts recusam sobrescrever a primeira avaliação.

```fish
set C $FRX_V7_CACHE
set R $HOME/.cache/frx-ai-v6-known-replay-20261030
cd research/frx_ai/v7

$FRX_PY -m unittest test_contracts              # 15 contratos; não medem linguagem
$FRX_PY freeze.py $C --verify                   # gerador congelado sem alterações
python3 waterfall.py $R                         # Experimento A sobre previsões de v6
$FRX_PY oracle.py $R                            # Experimento B em P6
$FRX_PY oracle.py $C --gold p7-gold.jsonl --prefix p7m
$FRX_PY oracle.py $C --gold j7-gold.jsonl
$FRX_PY develop.py $R --threshold 3.4           # desenvolvimento, conhecido
python3 posthoc.py $C                           # leitura de erros
$FRX_PY selector.py $C --old $FRX_V2_CACHE      # validação cruzada de D7
$FRX_PY benchmark.py $C                         # custo
```

Uma nova rodada de `final.py`, `selector.py --final` ou `c7.py` exige outro diretório, pois os resultados
já existem. Para isso: `collect.py` baixa o corpus (limitado à API pública do GitHub via `gh`),
`reserve.py` e `reserve_joint.py` reservam, `freeze.py` congela, os rótulos são escritos e travados com
`final.py --lock-labels`, e só então `final.py` roda. **Repetir nos mesmos dados é replay, não teste.**

`c7.py --replay` reproduz C7 depois das correções pós-teste e é marcado como conhecido.

## O que é experimental e o que não funciona

- Extração de relações: 9/16 corretas em teste novo, com falsos sucessos. Não usar como verdade.
- Procedimentos conjuntos: 2/20 recuperados. A forma da saída preserva a incerteza; o conteúdo erra.
- Próxima verificação: não superou "sempre pedir o log" contra o que mantenedores pedem.
- **Logs anexados por link não são lidos**: forneça o arquivo com `--log`.
- A recuperação não detecta "nada relevante", e as fontes podem ser fracas.
- Termos em português na CLI nunca foram avaliados; C7 é em inglês.
- Sem pesquisa na internet, ProtonDB, WineHQ ou documentação no índice.
- Nada foi verificado em um jogo real; nenhuma correção foi aplicada.
- Depende de caches de v2, v5 e v6; não há integração com Rust nem modelo no instalador.
