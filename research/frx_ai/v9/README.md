# FRX AI v9 — Autonomous Technical Research

Laboratório experimental. **Pesquisa de verdade em fontes públicas, mas ainda não conduz uma investigação confiável:** na avaliação
congelada, pesquisar de forma iterativa não superou uma pesquisa única. Só lê, nada instala ou executa, e não está no aplicativo.

Resultados: [REPORT.md](REPORT.md). Regras fixadas antes: [PROTOCOL.md](PROTOCOL.md). Fontes: [LITERATURE.md](LITERATURE.md).

## O que faz cada parte

| Parte | Arquivo | Origem da decisão |
|---|---|---|
| Rede: hosts, rotas, limites, cache, registro | `net.py` | Regra, imposta fora de qualquer decisão |
| GitHub ao vivo e snapshot congelado | `sources.py` | Busca lexical do GitHub; BM25 no snapshot |
| Fatos do problema, leitura, aplicabilidade, hostilidade | `reading.py` | Regra; vocabulário técnico escrito à mão |
| Consultas, pistas, parada, hipóteses, relatório | `engine.py` | Regra |
| Próxima verificação | `engine.py`, catálogo do v7/v8 | Regra |
| Reordenação opcional | `evaluate.py` (condição S) | **MiniLM público congelado**, sem rótulos do FRX |

**Nenhum serviço de IA é chamado, e nenhum modelo foi treinado.** O Claude escreveu este código e os rótulos de avaliação, mas não participa da
execução. Sem modelo gerativo, a "decisão" do protótipo é uma regra, e nada aqui é política neural aprendida.

## Usar no Fish

Na raiz do repositório. Só a biblioteca padrão é necessária.

```fish
set FRX research/frx_ai/v9/frx_research.py

# Ao vivo, na API pública do GitHub, sem credenciais
python3 $FRX --game "Batman: Arkham City GOTY" --app-id 200260 --gpu NVIDIA --problem "Poor performance with PhysX"

# Com rastreamento de cada leitura
python3 $FRX --game "Batman: Arkham City GOTY" --gpu NVIDIA --problem "Poor performance with PhysX" --trace

# Interativo: pergunta os campos
python3 $FRX

# Sem rede, no snapshot congelado (cerca de 2 s e 550 MB)
python3 $FRX --offline --game "Some Game" --gpu AMD --problem "black screen with mesa"

# Saída JSON, com o registro das chamadas
python3 $FRX --game "Some Game" --problem "crash on launch" --json

# Comparar estratégias: local, single, single-broad, fixed, random, iterative
python3 $FRX --strategy single --game "Some Game" --problem "crash on launch"

# Ignorar fontes posteriores a uma data
python3 $FRX --until 2024-01-01 --game "Some Game" --problem "crash on launch"

# Log inventado de teste. Qualquer outro arquivo é recusado
python3 $FRX --offline --game "Some Game" --problem "The game does not start" --fixture missing-dll.log
```

Sem credenciais, o GitHub permite 10 buscas por minuto e 60 leituras por hora, com limites secundários adicionais. Quando um limite é atingido, o
protótipo para, explica no relatório e não insiste.

## Reproduzir

```fish
set VP $HOME/.cache/frx-ai-rnd-v6-20261030/venv/bin/python
cd research/frx_ai/v9
python3 -m unittest test_security            # 31 testes, sem rede
python3 multiturn.py                         # continuidade entre turnos
python3 posthoc.py --stages                  # onde cada resposta se perdeu
python3 posthoc.py                           # replays de diagnóstico
$VP evaluate.py judged                       # julgamentos cegos
python3 live.py                              # ao vivo; espera os limites
```

`evaluate.py first` recusa rodar de novo: **repetir nos mesmos casos é replay, não teste.** Corpo dos dados e cache ficam em
`~/.cache/frx-ai-v9-20261031`; `snapshot/` traz métricas, hashes e rótulos sem texto dos comentários.

`net.py` mudou depois da leitura congelada, só para espaçar buscas ao vivo entre processos. A busca congelada não usa a rede, então isso
não afeta aqueles números, mas `freeze9.py --verify` agora aponta essa diferença.

## Camadas de conhecimento

| Camada | Onde | Regra |
|---|---|---|
| Em tempo de execução | Memória da investigação | Some ao terminar |
| Cache de fontes | `~/.cache/frx-ai-v9-20261031/net` | Cópia da resposta, com data; não é conhecimento verificado |
| Memória do caso | `engine.State` | Só deste caso; relatos seguem como relatos |
| Conhecimento persistente verificado | **Não existe** | Nada é promovido sem validação |
| Parâmetros | **Nada muda** | Pesquisar não ensina o modelo |

Como evita informação inadequada: compara fabricante de GPU, marca textos antigos e threads de outros jogos, e mantém a data de cada fonte.
**São sinais simples, com erros, e o cache não expira sozinho.**

## O que não funciona

- A pesquisa iterativa não supera a pesquisa única nos conceitos que chegam ao relatório.
- O leitor perde quase tudo: mesmo com a resposta como consulta, o conceito chega ao relatório em 1 de 17 casos.
- Só cerca de 20% dos documentos lidos parecem relevantes, em julgamento só pelo título.
- O vocabulário técnico é uma lista fixa, e termos fora dela quase não viram pista.
- Pistas podem enganar: em 5 de 9 relatórios provisórios, as principais levavam para o lado errado.
- A busca ao vivo usa só `ValveSoftware/Proton` na prática. Outros repositórios só entram se uma pista os aciona.
