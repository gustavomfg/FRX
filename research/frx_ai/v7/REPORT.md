# FRX AI v7 — Diagnostic Reasoning & Candidate Discovery

Relatório de 30/10/2026. Laboratório isolado: nenhum arquivo do aplicativo, da Steam,
de jogos, prefixos ou drivers foi alterado. Sem commit, tag ou push.

**Resultado em uma frase.** O gargalo de candidatos foi localizado e reduzido em dados
novos (cobertura de 4/30 para 13/30 e de 10/67 para 28/67), mas procedimentos conjuntos
não melhoraram, e nenhum método aprendido superou "sempre pedir o log" na escolha da
próxima verificação.

Todos os rótulos de P7, J7 e C7 são provisórios, produzidos por este agente, sem revisão
humana independente. O mesmo agente escreveu gerador e rótulos, o que pode favorecer o
método mesmo com a ordem congelamento → anotação → previsão.

## Hipóteses

| ID | Hipótese | Resultado |
|---|---|---|
| H1 | As perdas de v6 estão antes do verificador | **Confirmada**, com causas diferentes do esperado |
| H2 | Com candidatos corretos o verificador recupera muito mais que 9/70 | **Confirmada**: 41/70 a 57/70 |
| H3 | Separar proposta de decisão eleva a cobertura em dados novos | **Confirmada** dentro do critério; ganho menor que no desenvolvimento |
| H4 | Representar o procedimento recupera conjuntos | **Refutada**: 2/20 contra 3/20 de v6 |
| H5 | Seleção aprendida da próxima verificação supera os baselines | **Refutada** contra a maioria; superou só as regras fixas |

## Dados e protocolo

O [protocolo](PROTOCOL.md) foi registrado antes de coletar dados novos.

| ID | Unidade | Tamanho | Estatuto |
|---|---|---|---|
| P6 | trechos de v6 | 96 trechos, 70 relações | **Agora conhecido**; usado no Exp. A/B e no desenvolvimento |
| P7-M | parágrafos de 8 discussões novas | 60 unidades, 30 relações, 0 conjuntas | Teste novo |
| P7-J | mesmas discussões, 3+ verbos | 17 unidades, 17 relações, 3 conjuntas | Desafio; quase não tem procedimentos reais |
| J7 | relatos com várias alterações | 40 unidades de 36 discussões, 67 relações, 20 conjuntas | Desafio enriquecido, selecionado depois do congelamento |
| D7 | pedido de mantenedor após um relato | 532 (417 treino, 115 teste), 340 grupos | Rótulos por regra sobre palavras do mantenedor |
| C7 | primeiro post de discussões novas | 24 casos | Marcos do agente mais o pedido real do mantenedor (9 casos) |

Corpus: instantâneo público de ValveSoftware/Proton com 9.900 issues e 30.000 comentários
(outubro de 2023 a outubro de 2026), fora do Git. P7, J7 e C7 não compartilham AppID com
nenhum conjunto de v1–v6 nem entre si. D7 foi dividido por AppID.

Ordem cumprida: reservar sem exibir → congelar por hash → anotar → travar rótulos → prever
uma vez. **Emenda registrada** em [`snapshot/amendments.json`](snapshot/amendments.json): a
primeira execução abortou antes de qualquer métrica por um intervalo invertido em código
herdado de v5; `final.py` passou a ordenar os dois offsets, sem tocar nos arquivos congelados.

Comparabilidade com v5/v6: o protocolo de P6 foi preservado (IoU ≥0,5, um para um, grupo
completo) e o avaliador de v6 foi reutilizado sem alteração. Todos os comparadores rodaram
em P7 e J7. Números de P6 e P7 **não** são somados. A variante leniente (verbo leve) é
publicada e mudou no máximo uma relação em cada conjunto.

## Experimento A — onde as relações de v6 se perderam

Previsões publicadas de v6, 70 relações. Causa da **primeira** falha ao longo do fluxo:

| Etapa | Relações |
|---|---:|
| Evento excluído pelo filtro de modalidade | **25** (11 anotados como executados, 7 preexistentes, 7 desconhecidos) |
| Evento não detectado | **13** |
| Resultado não detectado | **11** |
| Gatilho em outra cabeça ou fronteira (`Tried`/`fiddling`) | **9** |
| Resultado detectado com a classe errada | 1 |
| Par ou grupo não proposto, com todas as peças presentes | **0** |
| **Subtotal perdido antes do verificador** | **59** |
| Rejeitadas pelo verificador | 2 |
| Recuperadas | 9 |

Contando todas as causas de cada relação: resultado não detectado em **41/70**, modalidade em
29, gatilho não detectado em 13, fronteira em 10, classe errada em 3.

Três respostas diretas:
- **Formar pares não era o problema.** Nenhuma relação caiu nessa etapa, porque todas caíam antes.
- **Identidade não eliminou candidatos.** O gerador não filtra por identidade; ator divergente
  aparece em só 2/70. Identidade é erro de decisão, não de cobertura.
- **Argumento incompleto não eliminou candidatos**, pois o gerador não exige argumento. A perda
  nesse caso aparece como fronteira de gatilho.

Das 7 conjuntas, nenhuma foi proposta, e todas falharam nas etapas anteriores ao agrupamento.

## Experimento B — limite do verificador com candidatos-oráculo

Diagnóstico isolado em [`oracle.py`](oracle.py); um teste garante que a inferência não importa
esse módulo. Entrada: eventos e resultados anotados, com o grupo correto e cada evento isolado
como distratores. Formato: corretas/aceitas (falsos sucessos).

| Verificador | P6 (70) | P7-M (30) | J7 (67) | Conjuntas J7 (20) |
|---|---:|---:|---:|---:|
| v6 L1 com filtro literal | 41/50 (3) | 16/18 (0) | 45/58 (6) | 13 |
| v6 L1 sem filtro literal | 57/78 (6) | 24/33 (3) | 50/68 (8) | 13 |
| v6 estrutural sem filtro | 53/79 (10) | 24/33 (3) | 40/71 (12) | 3 |
| v7 estrutural | 54/59 (2) | 22/24 (0) | 40/46 (1) | 8 |

**Com o mesmo L1 de v6, 9/70 vira 41/70 ou 57/70.** O filtro literal custa 16 relações em P6,
principalmente estados preexistentes. O valor de v7 em P6 é otimista, pois as regras foram
desenvolvidas nesses dados; em P7-M e J7 o decisor já estava congelado, mas a tarefa do oráculo
continua fácil, com poucos distratores. Isso é teto diagnóstico, não desempenho alcançável.

O L1 escolhe grupos melhor que as regras de v7 (13 contra 8), mas aceita mais erros (6 contra 1).

## Experimentos C, D e E — teste novo

Mesmo protocolo e mesmas unidades. "Candidatos" = relações verdadeiras que chegam completas ao decisor.

| P7-M, 60 unidades, 30 relações | v5 Stanza | v6 selecionado | **v7** | Candidatos v7 + L1 v6 |
|---|---:|---:|---:|---:|
| Candidatos completos | n/a | 4/30 | **13/30** | 13/30 |
| Candidatos por documento (média; mediana) | n/a | 0,1; 0 | 1,0; 0 | 1,0; 0 |
| Precisão dos candidatos | n/a | 57% | 23% | 23% |
| Relações corretas/propostas | 1/1 | 2/3 | **9/16** | 10/26 |
| Recall de relações | 3,3% | 6,7% | **30,0%** | 33,3% |
| Falsos sucessos | 0 | 1 | **3** | 10 |
| Execuções corretas/afirmadas (de 29) | 8/11 | 11/15 | 13/16 | 13/16 |

| J7, 40 unidades, 67 relações | v5 Stanza | v6 selecionado | **v7** | Candidatos v7 + L1 v6 |
|---|---:|---:|---:|---:|
| Candidatos completos | n/a | 10/67 | **28/67** | 28/67 |
| Candidatos por documento (média; mediana) | n/a | 2,8; 1 | 7,5; 8 | 7,5; 8 |
| Precisão dos candidatos | n/a | 9% | 9% | 9% |
| Relações corretas/propostas | 5/17 | 8/27 | **14/32** | 20/66 |
| Recall de relações | 7,5% | 11,9% | **20,9%** | 29,9% |
| Falsos sucessos | 4 | 6 | **5** | 15 |
| Negativas corretas | 2 | 4 | **8** | 13 |

P7-J: 4/17 candidatos contra 1/17; 2/4 relações contra 1/1; zero falso sucesso nos dois.

**Critério pré-registrado.** Cobertura de pelo menos 2× a de v6: **cumprido** (3,25× em P7-M,
2,8× em J7, 4× em P7-J). Mediana de até 8 candidatos: **cumprido**, no limite em J7. Taxa de
falsos sucessos não maior que a de v6: **cumprido** (P7-M 3/16 contra 1/3; J7 5/32 contra 6/27).

Ressalvas importantes:
- Em P7-M, a contagem absoluta de falsos sucessos **subiu de 1 para 3**, e a precisão ficou em
  56% (Wilson 33–77%). A meta histórica de ≥90% não foi atingida.
- Bootstrap por discussão, ganho de v7 sobre v6. Cobertura: P7-M [0; 45] pontos percentuais, J7
  [13; 41]. Recall de relações: P7-M [0; 33], J7 [−3; 21]. **O ganho de cobertura é robusto só em
  J7; o de recall de relações inclui zero nos dois.** P7-M tem 8 discussões, e várias sem relações.
- O otimismo do desenvolvimento era real: em P6, já conhecido, a cobertura foi 48/70 (69%); em
  dados novos, 43% e 42%.
- O L1 de v6 sobre os candidatos novos recupera um pouco mais e erra muito mais (10 e 15 falsos
  sucessos). Treinado em outro gerador, ele não se transfere.

### Experimento D — procedimentos conjuntos

| J7, 20 relações conjuntas | v5 | v6 | v7 | v7 + L1 |
|---|---:|---:|---:|---:|
| Grupo completo proposto | n/a | 3 | **6** | 6 |
| Grupo completo recuperado | 2 | 3 | **2** | 3 |
| Crédito individual indevido (positivo) | 3 (1) | 1 (0) | 2 (1) | 2 (2) |
| Grupo parcial afirmado | 0 | 2 | 2 | 5 |

**A hipótese foi refutada.** Dobrar os grupos propostos não recuperou mais. Das 20: em 6 o trecho
do resultado não foi detectado; nas 14 restantes faltam membros, sobram ações ou o decisor prefere
uma ação isolada. Crédito individual indevido continua existindo: em `the bug disappeared`, com
`Jumped` e `went`, v7 atribuiu o resultado só a `went`. É um erro crítico, pois é justamente o que
a representação deveria impedir.

O que funciona é a **forma da saída**: toda afirmação de grupo carrega `structure`, `attribution`,
`individual_cause="unknown"` e `causality_confirmed=false`; nenhuma saída afirmou causalidade. Nas
duas conjuntas corretas de v7, a estrutura também estava certa (2/2), amostra pequena demais para
estimar acurácia. Vale também notar um caso que o desafio trouxe e o sistema não trata: `changed
together, so I cannot isolate which change cleared 0063`, em que o próprio autor nega a atribuição.

### Análise de erros

Primeira falha de v7 em J7 (67): rejeitada na decisão 13, ação não proposta com modalidade não
executada 14, fronteira 9, resultado não detectado 9, agrupamento 3, classe errada 2, não detectado 2,
recuperadas 15 (a tabela mostra 14 porque o avaliador conta cada afirmação uma só vez, um para um).

Falsos sucessos reais incluem:
- palavra `workaround` lida como melhora, em `didn't seem to have any effect` ([fonte](https://github.com/ValveSoftware/Proton/issues/5982#issuecomment-1854155681));
- objetivo lido como resultado, em `get ... working properly` ([fonte](https://github.com/ValveSoftware/Proton/issues/651#issuecomment-2512387203));
- `the problems started` lido como melhora, porque o padrão `the <palavra> started` trata qualquer sujeito como o jogo ([fonte](https://github.com/ValveSoftware/Proton/issues/2014#issuecomment-2183525775)).

A fraqueza é a mesma nos três: palavras de resultado sem verificar se a oração afirma um resultado
sobre o jogo. Nenhuma regra foi ajustada depois do teste; corrigir exigiria uma nova coorte.

## Frente B

### Protótipo de investigação

Implementado em [`investigate.py`](investigate.py), [`signals.py`](signals.py),
[`knowledge.py`](knowledge.py), [`retrieve.py`](retrieve.py) e [`frx_lab.py`](frx_lab.py):

```
caso → sinais tipados com citação literal → o que falta → hipóteses com a favor/contra
     → próxima verificação com motivo → você executa e informa → hipóteses mudam de estado
```

- 13 famílias de sinais de log e 12 de sintoma; 20 hipóteses; 28 verificações somente leitura ou
  que declaram alterar estado. Tudo escrito à mão e identificado como regra.
- Uma hipótese nunca vira causa: `confirmed_causes` é sempre vazio, e o estado máximo é "reforçada
  por verificação sua".
- Para `undefined symbol: SDL_fabsf` sem caminho, a primeira verificação é descobrir qual arquivo
  pede o símbolo; com caminho, de onde a biblioteca foi carregada. Não nomeia biblioteca nem sugere
  instalar nada (há teste para isso).
- Histórico por caso com o ambiente: verificação feita não se repete, e mudar GPU ou jogo descarta
  o histórico em vez de transferir.
- Recuperação: TF-IDF sobre 92.920 parágrafos, reordenação pelo recuperador FRX v2 preservado, uma
  fonte por discussão. Cada fonte mantém autoridade, ano, jogo, GPU e versão citada. Sem rede.

**Limitação relevante da recuperação:** no caso Batman, as fontes devolvidas têm pouca utilidade,
e a discussão histórica não aparece bem. Não há detecção de "nenhuma resposta relevante" (Q-007).
Ela não foi avaliada neste laboratório além do caso demonstrativo.

### Regras × recuperação × modelo × híbrido em demonstrações (D7)

Dado: o que mantenedores pediram em seguida, 532 pedidos reais. Espaço fechado de 10 verificações,
sem saída deste agente no treino. Divisão por AppID; seleção por validação cruzada; **teste lido
uma vez**. Duas contaminações de template foram encontradas e corrigidas no treino antes da leitura
(comentários "Issue transferred from" e dicas de `PROTON_LOG` no modelo de issue).

| Política, teste com 115 pedidos em 78 grupos | Top-1 | Top-3 | MRR | Recall macro |
|---|---:|---:|---:|---:|
| **Maioria ("sempre pedir o log")** | **58,3%** | 74,8% | 0,703 | 0,111 |
| Logística TF-IDF + estado, C=10 | 57,4% | **80,0%** | 0,700 | 0,179 |
| Recuperação de casos k=15 (selecionada na validação cruzada) | 54,8% | 73,9% | 0,677 | 0,115 |
| Logística TF-IDF, C=1 | 53,0% | 73,0% | 0,672 | 0,142 |
| Logística embedding v2 + estado | 39,1% | 68,7% | 0,575 | 0,147 |
| Híbrido com máscara "já fornecido" (TF-IDF + estado, C=10) | 47,8% | 73,9% | 0,633 | 0,195 |
| Regras fixas "pedir o que falta" | 26,1% | 47,0% | 0,415 | 0,175 |
| Logística só com estado | 26,1% | 60,0% | 0,480 | 0,150 |

Ganho do selecionado sobre a maioria, por grupo: [−8,8; +0,9] pontos percentuais. Sobre as regras
fixas: [+19,7; +37,0]. **H5 refutada contra a maioria.** O método neural (embeddings) foi pior que
TF-IDF, e por isso não foi adotado para esta decisão.

O dado explica o resultado. No treino, o mantenedor pede o log em 51,5% dos casos sem log e em
56,7% dos casos com log: o pedido quase não depende do estado que conseguimos extrair. E as regras
de "pedir o que falta", que parecem sensatas, acertam metade do que a maioria acerta.

Limite: pedido de mantenedor é **prática de triagem, não prova de utilidade**. Muitos são pedidos
de log para uma pessoa nova na conversa; um modelo aqui imita o processo, não o diagnóstico.

### Investigação completa (C7)

24 primeiros posts, sem logs anexados no texto. Critérios e marcos escritos antes de executar. O
pedido real do mantenedor existe para 9 casos.

| Critério | Sempre log | Regras v6 | **Regras v7** | Prior de casos v7 | Híbrido v7 |
|---|---:|---:|---:|---:|---:|
| Próximo passo, marcos do agente (top-1) | 7/24 | 7/24 | **8/24** | 7/24 | 4/24 |
| Próximo passo, marcos do agente (top-3) | 7/24 | 7/24 | **16/24** | 13/24 | 16/24 |
| Próximo passo, pedido do mantenedor (top-1) | **6/9** | **6/9** | 3/9 | **6/9** | 3/9 |
| Eficiência: não pede o já fornecido | 9/24 | 9/24 | **20/24** | 9/24 | 13/24 |
| Compreensão do sintoma | n/a | n/a | 20/24 | 20/24 | 20/24 |
| Hipóteses pertinentes | n/a | n/a | 19/24 | 19/24 | 19/24 |
| Citações literais | n/a | n/a | 24/24 | 24/24 | 24/24 |
| Nenhuma causa confirmada | n/a | n/a | 24/24 | 24/24 | 24/24 |
| Segurança (sem instalação nem execução) | n/a | n/a | 24/24 | 24/24 | 24/24 |

**As duas referências discordam**, e isso é um resultado: pelos marcos deste agente as regras v7
vencem; pelo que o mantenedor realmente pediu, "sempre log" vence. Os marcos do agente favorecem o
catálogo que o próprio agente escreveu. O desempenho em 9 casos não permite conclusão estatística.

Falhas encontradas na leitura pós-teste, **sem alterar o resultado acima**:
- `Video driver version:` do modelo de issue disparava o sintoma de vídeo, gerando
  `inspect-media-errors` como primeira verificação em 7/24 casos com regras;
- log anexado por link não contava como fornecido.

Corrigidos para a CLI e reexecutados como **replay conhecido, que não é avaliação independente**:
regras 13/24, híbrido 16/24, prior de casos 11/24 em top-1 pelos marcos; pelo mantenedor, regras
1/9, híbrido 2/9. A correção piorou a concordância com o mantenedor. A política padrão da CLI
continua `rules`, a melhor na leitura original; a vantagem do híbrido no replay precisa de casos
novos. O protótipo não lê o conteúdo de logs anexados: é a limitação prática mais importante, pois
é onde mora a evidência técnica.

## Ensino por modelo professor (exploratório, não treinado)

> **Errata de 30/10/2026:** os itens 1 e 2 estão preservados, mas erram a origem. Os rótulos de v1–v6 são do
> Codex, e os termos citados não são o ponto de partida para eles. Ver [auditoria](../audit/PROVENANCE-AUDIT.md)
> e RXAI E-001.

**Nenhum modelo professor foi usado.** Decisão fundamentada:

1. Os [Termos Comerciais da Anthropic](https://www.anthropic.com/legal/commercial-terms) (vigência
   17/06/2025, D.4) vedam acessar o serviço para construir produto concorrente, incluindo treinar
   modelos concorrentes. Não cabe a este laboratório decidir se a FRX AI se enquadra.
2. **Isso afeta o trabalho já feito.** Os rótulos de v2–v6 são "provisórios do agente", e parte
   deles treinou modelos, inclusive o L1 de v6. No v7, saída do agente só avalia. **A decisão sobre
   os modelos anteriores é do responsável pelo projeto.**
3. Pesos abertos foram descartados por falta de hipótese mensurável: Qwen já falhou nesta tarefa,
   e este laboratório mostrou que falta medida de utilidade, não um professor.
4. As funções de rótulo de D7 foram escritas por este agente; os dados são palavras humanas. É uma
   zona cinzenta registrada, e o seletor treinado não foi adotado em inferência padrão.

A substituição testada foi **demonstração humana pública**, e o resultado é informativo: ela ensina
triagem. Um professor com rótulos plausíveis aprenderia o mesmo viés sem que a acurácia o revelasse.

## Custo

CPU Ryzen 5 4600G; RTX 4060 detectada, não usada (venv sem CUDA). Nenhum peso novo baixado e nenhuma
rede neural treinada; logísticas de D7 treinam em segundos.

| Medida, processo novo | Valor |
|---|---|
| Extração v7, parse novo, mediana | 9,4 ms por unidade |
| Extração v7, parse em cache, mediana; P95 | 2,6 ms; 8,0 ms (v6 reportou 14,7 ms e 27,5 ms em outra coorte) |
| Inicialização; primeira requisição | 3,3 s; 1,6 s |
| RSS, extração | 340 MiB |
| CLI completa (fontes, reordenação, prior) | 4,2 s, 731 MiB |
| CLI só com regras | 0,06 s, 20 MiB |
| Código novo de inferência | 61 KB |
| Parser spaCy; recuperador v2; índice de evidências | 15 MB; 109 MB; 63 MB |

O caminho só com regras cabe em qualquer integração; o índice e o reordenador são o custo real. Os
tempos de v6 e v7 têm coortes diferentes e não são comparação direta. Python e dependências não
entram nesses tamanhos.

## Descobertas

1. **O verificador nunca foi o gargalo.** O mesmo L1 passa de 9/70 a 41–57/70 com candidatos
   corretos. Quatro laboratórios de classificadores miravam a etapa errada.
2. **Detectar resultados é a maior lacuna**: afeta 41/70, mais que modalidade ou gatilho.
3. **Separar proposta de decisão funciona em dados novos**, com ganho real e menor do que o
   desenvolvimento sugeria.
4. **Formar o grupo certo é outro problema**: proposto não significa recuperado, e o crédito
   individual indevido persiste. Representar a incerteza é fácil; decidir o grupo, não.
5. **Pedidos de mantenedor ensinam triagem, não diagnóstico.** A hipótese de que uma política
   aprendida bastaria foi refutada: falta um rótulo de utilidade, não um modelo.
6. **Texto de template é um contaminador recorrente**: apareceu três vezes (rótulo, estado, sintoma).
   Duas foram pegas no treino; a terceira só no teste. O FRX real enfrentará o mesmo.

## Limitações

Rótulos do agente sem revisão independente. Gerador e rótulos do mesmo autor. P7-M tem 8 discussões
e nenhuma relação conjunta; J7 é enriquecido por regra lexical. C7 tem marcos do agente, só primeiro
post e logs não lidos. D7 mede imitação. Português da CLI sem avaliação. Só comentários em inglês
em ValveSoftware/Proton, sem ProtonDB, WineHQ ou documentação no índice. Recuperação sem detecção de
ausência de resposta. Nada foi executado em um jogo; nenhuma correção foi verificada.

## Próxima hipótese científica

**Rotular utilidade, não pedidos.** Uma verificação é útil quando seu resultado muda o que se faz em
seguida. As discussões trazem isso: pedido do mantenedor, resposta do usuário e conclusão posterior
("fixed in", fechamento, causa nomeada). A pergunta é: *dado o estado de um caso, quais verificações
historicamente separaram hipóteses e levaram a uma conclusão?* Isso exige anotar trajetórias
completas, com revisão humana, e ler logs de verdade, em vez de depender do primeiro post.

Em paralelo: um detector de resultados (afirmação de estado do jogo por oração) avaliado sozinho em
coorte nova, porque é a maior perda medida, e revisão humana independente de P7 e J7.
