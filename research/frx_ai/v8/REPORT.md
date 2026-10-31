# FRX AI v8 — relatório

Data: 30/10/2026. Regras fixadas antes dos dados: [PROTOCOL.md](PROTOCOL.md). Métricas sem texto: [`snapshot/`](snapshot/).

**Todos os números usam rótulos H0, escritos por este agente (Claude Opus 5.5). Não são validação humana independente.**
Nenhum modelo foi treinado. Tudo o que o v8 acrescenta é regra.

## Resultado em uma página

| Frente | Critério do protocolo | Resultado | Leitura |
|---|---|---|---|
| O | Menos falsos positivos que o v7 | 3 contra 6 | Atendido |
| O | Cobertura ≥ v7 − 5 pontos | 68,8% contra 87,5% | **Não atendido** |
| O | Classe pareada contra o v7, IC acima de 0 | +25,9 pontos [10,3; 41,4] | Atendido |
| J | Mais grupos que o v7, sem atribuição individual falsa | 1/14 contra 0/14; 0 falsas | Atendido só no papel: **falhou na prática** |
| T | Top-3 pareado acima do melhor comparador | 25/26 contra 10/26 | Atendido, mas **muito menor após a análise de sensibilidade** |
| T | Causa sem suporte = 0 | 0 | Garantido pela construção, não medido |
| T | Pedidos redundantes ≤ regras do v7 | 1/26 contra 12/26 | Atendido |
| L | Sinais relevantes acima da busca literal | Trechos: 3/7 contra 6/7 | **Não atendido** |

**O que isso significa:**

1. Separar estado de resultado funcionou: menos afirmações falsas e classe melhor. Mas a lista de regras cobre mal a linguagem real.
2. O problema dos procedimentos conjuntos não é de representação. É de extração.
3. A vantagem na próxima verificação vem principalmente de notar que já existe um log. **Não é diagnóstico inteligente.**
4. O ponto mais fraco para investigar casos reais é o acompanhamento da conversa: só 7 de 18 resultados anotados foram encontrados.

## Frente O — detecção isolada de resultados

**Descoberta anterior ao teste:** medido sozinho, o detector do v7 parecia ter muitos falsos positivos nos gabaritos
antigos, mas boa parte era sintoma real. Esses gabaritos só marcam resultados ligados a uma ação, então **não medem a
precisão de um detector isolado**. O v8 definiu a tarefa no [guia](../benchmark/ANNOTATION-GUIDE.md) e criou uma coorte nova.

**Coorte:** 60 comentários novos, um por thread, sorteados depois do congelamento. Dois rotulados `unclear` ficam fora,
restando 58 unidades e 32 afirmações.

| Sistema | Natureza | Afirmações achadas | Precisão de afirmação | Positivas | Falsas positivas | Classe (com `positive` fundido) |
|---|---|---:|---:|---:|---:|---:|
| **v8** | Regras | 22/32 | 22/30 | 15 | 3 | 37/58 |
| v7 | Regras sobre parser | 28/32 | 28/47 | 13 | 6 | 22/58 |
| `weak` do v2 | Regex | 7/32 | 7/10 | 10 | 5 | 28/58 |
| E07 seletivo | Pesos congelados | 0/32 | n/a | 0 | 0 | 26/58 |
| E07 sem filtro | Pesos congelados | 17/32 | 17/21 | 6 | 2 | 32/58 |
| Sempre `no_claim` | Referência | 0/32 | n/a | 0 | 0 | 26/58 |

Diferença de classe do v8, IC de 95%:

| Contra | Diferença | IC |
|---|---:|---|
| v7 | +25,9 pontos | [10,3; 41,4] |
| `weak` | +15,5 pontos | [0,0; 32,8] |
| E07 sem filtro | +8,6 pontos | [−6,9; 24,1], **sem diferença clara** |
| Sempre `no_claim` | +19,0 pontos | [3,4; 36,2] |

No v8, `resolved` teve 12 acertos em 15 (IC 54,8–93%), muito longe do limite de 0,90 usado no plano.

**Queda do desenvolvimento para o teste:** a cobertura caiu de 16/17 para 22/32, e os falsos positivos subiram de 0 para 3.
As regras se ajustaram demais aos casos que li.

**Erros do teste:**

- **Falso `resolved`:** escopo de negação ("neither installing A or B fixed it"), outro jogo ("Tekken 8's online issue got
  resolved") e relato de terceiros dentro de uma pergunta.
- **Afirmações perdidas:** frases legítimas fora da lista: "this fix worked for me too", "made the game work for me",
  "had success with both", "results exactly the same", "do absolutely nothing for me", "no joy".

**Conclusão:** H-O1 é parcialmente sustentada, com menos falsos positivos e menor cobertura. H-O2 não se sustenta com folga: o
E07 sem filtro não fica claramente atrás das regras. Mais regex não parece a saída. Antes de qualquer nova tentativa, é preciso
um gabarito humano.

## Frente J — procedimentos conjuntos

**H-J1 refutada.** A representação não era o problema: o esquema do v8 expressa os 20 grupos do J7, mas o `structure` do v7 também
expressava. Nos 20, a relação é sequência (11), conjunta (7) ou alternativas (2). O que falha é encontrar os membros e o resultado.

| Medida | v8 | v7 |
|---|---:|---:|
| Grupos corretos, em 14 | **1** | 0 |
| Relação correta | 1 | 0 |
| Grupos inventados | 3 | 3 |
| Atribuição individual falsa | 0 | n/a |

**Em desenvolvimento conhecido** (J7, 40 unidades), o v8 fez 4 de 20, contra 2 de 20 do v7. Não houve generalização.

**Por que falhou:** 10 dos 14 grupos são alternativas, e vários se perderam antes, porque o detector não achou o resultado. Listas
como "Tried A, B and C" perdem membros quando o verbo está longe, e sequências entre frases raramente são reconhecidas.

**O que fica de útil:** a representação não credita um único membro de um grupo. Uma linha de inicialização com várias opções é
tratada como grupo, e um resultado de grupo chega às verificações como inconclusivo.

## Frente T — trajetórias completas

**Coorte:** 12 threads novos, com 26 pontos de decisão. Em cada um, as políticas veem só o que já foi dito.

Foram anotados 63 passos: 12 problemas, 10 evidências, 6 hipóteses, 15 verificações, 18 resultados e 2 decisões. Os threads reais
quase nunca seguem o esquema completo em ordem, e **hipóteses explícitas são raras** (6 em 12 threads).

| Política | Top-1 | Top-3 | Redundante | Mudou quando devia |
|---|---:|---:|---:|---:|
| **Separar hipóteses (v8)** | 21/26 | 25/26 | 1 | 6/12 |
| Sempre pedir o log | 7/26 | 8/26 | 14 | 4/12 |
| Regras do v7 | 2/26 | 10/26 | 12 | 5/12 |

Diferença pareada por thread no top-3: +65,4 pontos [41,7; 85,7] contra sempre-log, +57,7 [33,3; 80,8] contra o v7.

### Por que este resultado não sustenta "diagnóstico inteligente"

**A análise posterior mostra forte dependência de uma regra.** Em 16 das 26 decisões, a escolha foi `read-attached-log`. Essa
verificação foi criada no v8, os comparadores não podem emiti-la e o mesmo autor escreveu política e gabarito.

| Análise posterior | v8 | Sempre-log | Regras v7 |
|---|---:|---:|---:|
| Tratando pedir e ler log como a mesma coisa: top-1 | 21/26 | 18/26 | 2/26 |
| O mesmo: top-3 | 25/26 | 19/26 | 17/26 |
| Só decisões em que ler o anexo não é aceitável (13): top-1 | 8/13 | 5/13 | 2/13 |

Com a fusão, a diferença de top-1 contra sempre-log cai para +11,5 pontos [0,0; 29,2], e a de top-3 fica em +23,1 [4,3; 42,9].

**H-T1 se sustenta contra as regras do v7, mas contra "olhar o log" ela é modesta e frágil.** O ganho mais claro é simples e útil:
não pedir de novo o que a pessoa já forneceu. Nenhum modelo aprendeu nada aqui.

**Mais limites:**

- A política recomenda ler o anexo, mas **o protótipo não lê anexos**.
- **H-T2 não se sustenta:** a decisão mudou corretamente em 6 de 12 casos, sem vantagem real.
- **Acompanhar a conversa é o elo fraco:** de 18 resultados anotados, foram encontrados 7, com 31 extraídos. A maior parte das
  extrações é ruído.
- O gabarito tem em média 2,38 verificações aceitas entre 30, escolhidas pelo mesmo autor.

## Frente L — logs

Oito trechos colados em comentários públicos, redigidos. Os anexos não foram baixados.

| Medida | Regras | Busca literal |
|---|---:|---:|
| Tipo de sinal esperado | 3/4 | Não classifica |
| Trechos relevantes encontrados | 3/7 | 6/7 |
| Linhas exibidas, de 60 | Só sinais | 20 |
| Sinais extras | 0 | n/a |
| Causa inventada | 0, pela construção | n/a |
| Módulo correto | 1/2 | n/a |

Nos quatro trechos só com ruído (`fixme`, `trace`), as regras não apontaram sinal falso, mas a busca literal também não.

**H-L1 não se sustenta.** As regras dão tipos, mas perdem trechos que uma busca simples acha. Com n = 8, isto é indicativo. As 10
fixtures passam, mas foram escritas pelo mesmo autor: isso é teste de contrato, não medida.

## Simulador

Em 9 cenários sintéticos: 0 perguntas repetidas e a causa simulada sempre entre as apoiadas, porém com em média 2 hipóteses
apoiadas por cenário. **O resultado é circular** e só mostra que o ciclo não está quebrado. Fica fora dos critérios.

## Origem de cada capacidade

| Capacidade | Regra | Recuperação | Aprendido | Estado |
|---|---|---|---:|---|
| Detector de resultados | Sim | Não | Não | Experimental |
| Grupos de procedimentos | Sim | Não | Não | Falha na extração |
| Hipóteses e verificações | Sim | Não | Não | Experimental |
| Sinais de log | Sim | Não | Não | Experimental |
| Busca de evidências (v7, TF-IDF e R02) | Não | Sim | Só R02, congelado | Não usada na CLI do v8 |
| E07, comparador | Não | Não | Sim, congelado | Só comparação |

## Limites

- Rótulos H0 de um único agente, que também escreveu as regras. A separação em etapas reduz, mas não elimina, esse viés.
- Amostras pequenas, com intervalos largos.
- As coortes têm filtro de vocabulário. A taxa de afirmações não representa os comentários em geral.
- Quase só inglês. Português não foi avaliado.
- Nada foi testado em um jogo real, nem foi aplicada correção.
- `reports.rs` e o `outcome` do v3 não foram comparados, porque exigem a ação já dada.

## Próximo passo

1. Levar o piloto a pessoas: nada aqui substitui isso.
2. Medir o detector no gabarito humano, sem mais regex antes disso.
3. Melhorar o acompanhamento da conversa, medido por resultado encontrado por turno.
4. Decidir sobre anexos públicos: sem lê-los, a principal recomendação do sistema fica sem execução.
