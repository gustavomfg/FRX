# RXAI — FRX AI Research Journal
### Registro Científico de Pesquisa, Experimentação e Desenvolvimento

**Edição:** 5 · **Criado em:** 30/10/2026 · **Atualizado em:** 31/10/2026 · **Cobertura:** v1–v9.

RXAI é o diário científico da FRX AI. Não designa um modelo, uma arquitetura ou
uma versão do aplicativo. Este documento consolida os registros locais e aponta
para os artefatos detalhados; não substitui os relatórios nem os congelamentos.

**Conclusão atual:** recuperação especializada apresenta evidências favoráveis.
O v7 localizou o gargalo de candidatos e o reduziu em dados novos, mas procedimentos
conjuntos não melhoraram e nenhuma política aprendida superou o baseline de próxima
verificação. Interpretação de procedimentos e diagnóstico autônomo continuam
experimentais. Não há validação humana independente documentada dos rótulos de avaliação.

**Depois do v8:** separar estado de resultado reduziu afirmações falsas, com menor cobertura. Procedimentos conjuntos continuam
falhando, agora identificados como problema de extração. A vantagem na próxima verificação vem principalmente de notar logs já
fornecidos. Tudo no v8 é regra, com rótulos H0.

**Depois do v9:** o protótipo pesquisa de verdade em fontes públicas, mas a investigação iterativa não superou uma pesquisa única com orçamento igual. O gargalo
identificado é entender o que foi lido, e não encontrar documentos. Pesquisar não é o mesmo que raciocinar.

## Navegação

[I. Identidade](#identidade) · [II. Origem](#origem) ·
[III. Problema e critérios](#problema) · [IV. Laboratórios](#historico) ·
[V. Linha do tempo](#linha-do-tempo) · [VI. Estado atual](#estado-atual) ·
[VII. Descobertas](#descobertas) · [VIII. Questões abertas](#questoes) ·
[IX. Referências](#referencias) · [X. Reprodução e lacunas](#reproducibilidade) ·
[XI. Manutenção](#manutencao) · [XII. Changelog](#changelog) ·
[XIII. Proveniência](#proveniencia).

<a id="identidade"></a>
## I. Identidade da pesquisa

| Campo | Definição |
|---|---|
| Pesquisa | FRX AI — Specialized Linux Gaming Intelligence |
| Diário científico | RXAI |
| Domínio | Linux Gaming: Steam, Proton, Wine, tradução gráfica, drivers e dependências |
| Propósito | Investigar problemas de compatibilidade, estabilidade, desempenho e recursos de jogos Windows no Linux |
| Execução pretendida | Local, compacta, com potencial de integração ao backend Rust |
| Papel futuro | Auxiliar o Complete Experience Pack com evidências, contexto, hipóteses e histórico |
| Limite | Previsões de pesquisa não autorizam execução nem substituem validações do Fix Engine |

A hipótese orientadora é desenvolver competência especializada de investigação:
a FRX AI não precisa memorizar todas as soluções. Precisa localizar evidências,
interpretar acontecimentos, preservar requisitos e aprender com resultados no
contexto correto. Competir com um assistente generalista não é o objetivo.

O programa de pesquisa separa componentes aprendidos e algoritmos explícitos.
Recuperadores, classificadores e encoders ajustados aprendem parâmetros;
restrições, parsers de determinados erros, memória e verificações de integridade
podem ser determinísticos. Cada capacidade deve ser avaliada por seu efeito,
sem exigir uma rede neural onde uma solução simples seja suficiente.

<a id="origem"></a>
## II. Origem do projeto

A [investigação Batman/PhysX](BATMAN-PHYSX-INVESTIGATION.md), datada de 27/10/2026,
documenta a motivação declarada: um problema envolvendo Batman: Arkham City GOTY
(AppID `200260`), NVIDIA/PhysX e um arquivo que o usuário teria instalado
manualmente. O arquivo histórico exato permanece não identificado. Os valores
informais de FPS mencionados pelo usuário não são benchmarks registrados do FRX;
este diário não os converte em medições nem identifica o patch por suposição.

O registro local encontrou pistas, como referências a `SveSop/nvidia-libs`, mas
não confirmou uma receita equivalente à experiência histórica. Também mostrou
um risco de atribuição: instruções que mencionam Arkham City podem ter como
destino Arkham Asylum. Nome de tecnologia, GPU ou jogo na mesma página não
estabelece aplicabilidade.

Essa necessidade de investigar fontes, componentes e procedimentos motivou a
FRX AI. Os primeiros ensaios documentados de interpretação de relatos estão em
[FRX-AI-AVALIACAO.md](FRX-AI-AVALIACAO.md), de 29/10/2026. A origem é motivação
de projeto e histórico técnico; não constitui demonstração causal de eficácia.

<a id="problema"></a>
## III. Problema científico e critérios de leitura

**Pergunta central:** como desenvolver uma inteligência compacta e local capaz
de combinar evidências técnicas, relatos comunitários, contexto do sistema e
histórico experimental para investigar e ajudar a resolver problemas de jogos
Linux?

| Subproblema | Resultado que precisa ser demonstrado |
|---|---|
| Recuperação de evidências | Encontrar e ordenar fontes pertinentes, reconhecendo ausência de resposta |
| Linguagem técnica | Distinguir sintomas, configurações, requisitos, negação e incerteza |
| Ações e acontecimentos | Identificar operação, alvo, direção, ator, modalidade e sequência |
| Associação ação–resultado | Vincular o resultado ao evento ou grupo completo correto |
| Identidade/contexto | Preservar jogo/AppID, autor, ambiente, versões e referências cruzadas |
| Diagnóstico | Formular hipóteses sustentadas e reconhecer causas desconhecidas |
| Próxima investigação | Escolher uma verificação útil sem pressupor a resposta |
| Planejamento seguro | Fornecer informação estruturada para validação posterior pelo Fix Engine |
| Aprendizado contextual | Reter tentativas sem transferir eficácia indevidamente entre ambientes |

Boa recuperação não demonstra boa interpretação. Uma associação relatada não
comprova causalidade, uma observação pública não é telemetria local e um plano
serializável não demonstra que a correção seja segura ou eficaz.

### Convenções de evidência e métricas

- **Observado no laboratório:** resultado registrado em avaliação, com o protocolo
  e as limitações respectivos. Todos os rótulos de referência existentes são
  provisórios do agente; “manual” nos registros antigos não implica revisão humana.
- **Hipótese:** explicação ou método a testar. **Inferência:** interpretação
  fundamentada dos registros, explicitamente limitada. **Plano futuro:** ensaio
  proposto, sem resultado atribuído.
- Precisão de execução = execuções corretas / execuções afirmadas; recall =
  execuções corretas / execuções de referência. Cobertura seletiva de comentários
  e recall de eventos são denominadores diferentes.
- Precisão de relações = relações corretas / propostas aceitas; recall =
  relações corretas / relações de referência. Uma relação conjunta exige os
  proprietários completos, sem distribuir crédito individual automaticamente.
- A meta histórica de precisão ≥90% com cobertura útil é uma ambição de
  engenharia. Critérios menores de elegibilidade no desenvolvimento não são
  aprovação de produção. Zero previsões deixa precisão indefinida.
- Replays de testes publicados são reproduções conhecidas. Selecionar outra
  hipótese usando seus resultados exige um novo teste independente.

Os conjuntos abaixo têm identificadores editoriais para evitar comparações
indevidas. Não são novos datasets criados nesta consolidação.

| ID | Avaliação original | Unidade, dimensão e protocolo |
|---|---|---|
| P0 | Antecedentes, 29/10 | Qwen: seis casos rotulados e lote de 30 textos; classificadores: 185 frases; Rust: testes A/B de 80/40 pares em issues distintas |
| P1 | Laboratório raiz, denominado v1 no README | 230 pares comentário–alvo: 100 treino, 50 dev, 80 finais; quatro jogos finais; seleção congelada |
| P2-R | Interpretação v2 | 96 propostas finais/oito jogos; avaliação condicionada ao gerador determinístico; não mede recall global de entidades |
| P2-I | Recuperação v2 | 96 comentários; 39 consultas respondíveis de 64, templates em inglês; variantes FP32/INT8 identificadas |
| P3 | Teste v3 | 95 comentários/seis discussões; propostas automáticas; recuperação com 31 consultas respondíveis de 48 |
| P4 | Teste v4 | 57 parágrafos/cinco jogos, 64 eventos, 33 execuções; IoU de caracteres ≥0,5 |
| P5 | Teste v5 | 72 trechos/seis discussões, 128 eventos, 52 execuções e 32 relações de referência; inclui menções a outros jogos |
| P6-M | Principal v6 | 72 trechos/seis novas discussões, 43 execuções e 48 relações; correspondência um a um e grupos completos |
| P6-C | Desafios v6 | 24 unidades adicionais nas mesmas seis discussões: duas com nome exato e 22 enriquecidas por contexto/citação |
| P7-M | Principal v7 | 60 parágrafos/oito discussões novas, 30 relações, nenhuma conjunta; reservado antes do congelamento, rótulos travados antes das previsões |
| P7-J | Desafio v7, mesmas discussões | 17 unidades com 3+ verbos, 17 relações, três conjuntas; relatado à parte |
| J7 | Desafio v7 de procedimentos | 40 unidades/36 discussões, 67 relações, 20 conjuntas; selecionado por regra após o congelamento, enriquecido |
| D7 | Demonstrações v7 | 532 pedidos de mantenedores (417 treino, 115 teste), 340 grupos; dividido por AppID; rótulos por funções determinísticas, prática de triagem, não utilidade |
| C7 | Casos v7 | 24 primeiros posts de discussões novas; marcos do agente e, em nove casos, o pedido real do mantenedor |
| O8 | Detecção isolada v8 | 60 comentários novos, um por thread, sorteados após o congelamento; 58 avaliados; rótulos H0 travados antes das previsões |
| T8 | Trajetórias v8 | 12 threads novos, 26 pontos de decisão, replay incremental; rótulos H0 travados antes das previsões |
| L8 | Logs v8 | 8 trechos colados em comentários públicos, redigidos, sem baixar anexos; rótulos H0 |
| R9 | Pesquisa v9, congelada | 24 casos novos, 17 documentados; snapshot real cortado na data de cada caso; até 3 buscas e 6 leituras; gabarito H0 travado antes das previsões |
| R9-L | Pesquisa v9, ao vivo | API pública, sem credenciais; 7 casos × 3 condições, incluindo Batman como demonstração conhecida; não reproduzível |

P6-M e P6-C somam 96 trechos, 74 execuções e 70 relações. O agregado é descritivo;
o enriquecimento e as citações repetidas impedem tratá-lo como prevalência geral.
Não há curva única de qualidade v1→v7: mudam tarefas, unidades e conjuntos. A partir
do v7, P6 é material conhecido e não serve mais como teste.

<a id="historico"></a>
## IV. Histórico científico — v1 a v9

<a id="lab-v1"></a>
### FRX AI v1 — antecedentes e primeiro protótipo local

**Objetivo.** Interpretar relatos sobre ajustes e verificar se modelos pequenos
conseguem reconhecer a ação e o resultado, além de preservar fontes.

**Hipóteses.** Um LLM local, uma cabeça supervisionada ou NLI poderiam superar
sentimento global; reduzir a tarefa a detectar relatos poderia ser mais viável.

**Fundamentação científica.** Representações de frases, NLI, classificação por
alvo e adaptação com poucos exemplos. SetFit foi pesquisado, sem implementação
de seu treinamento nesta etapa; LoRA não foi utilizado.

**Metodologia.** P0 reúne ensaios anteriores ao laboratório raiz. Em P1, pares
foram divididos por discussão/jogo; dados sintéticos ficaram no treino. Houve
anotação pelo agente a partir de comentários completos, sem segundo anotador.
O dev não tinha regressões e o teste final tinha somente duas.

**Implementação.** Antecedentes: Qwen3 0,6B/1,7B/4B quantizados em llama.cpp CPU,
logística NumPy e extrator Rust determinístico. Protótipo v1: MiniLM e
DeBERTa-xsmall congelados, cabeças lineares treinadas, TF-IDF, consenso e parser
de estados por regras. Não houve fine-tuning dos transformers em P1.

**Experimentos.** Quatro famílias/ciclos em P1: comparação de representações,
aumento sintético/decomposição, foco em frases e NLI/detecção de relatos. Não
foi localizado um total único auditável de ajustes incluindo todos os antecedentes.

**Resultados.** Em P0, a logística nas 185 frases teve acurácia 0,60, contra
0,55 da maioria, sem reconhecer pioras; os Qwen produziram falsas interpretações
e não justificaram adoção. A regra Rust teve 15/15 e 9/9 afirmações corretas
nos testes A/B, recuperando 15/23 e 9/16 relatos, com amostra pequena.

| P1, teste final de 80 pares | Corretos/aceitos | Cobertura | Falsos sucessos |
|---|---:|---:|---:|
| Detector aprendido de relato | 14/29 (48,3%) | 36,3% | Não é tarefa de polaridade |
| NLI: resultado + estado da ação | 3/15 (20,0%) | 18,8% | 4 |
| Baseline Rust existente | 3/4 | 5,0% | Não resumidos neste indicador |

A recuperação exploratória de 23 consultas/75 comentários deu P@3 de 0,406
para sobreposição lexical e 0,449 para MiniLM. Não era um teste final independente.

**Descobertas.** Um comentário pode conter várias ações com efeitos diferentes.
Mais contexto ou um modelo maior não asseguraram vinculação correta. O sucesso
da regra Rust em P0 não se manteve como evidência de precisão geral em P1.

**Falhas e limitações.** Negação, sugestões, estados invertidos, procedimentos
genéricos e objetivos mistos. Um comparador NLI de desenvolvimento foi invalidado
por normalização em memória; o detector selecionado usava atributos separados.
Crimson Desert/`descriptor_heap` é exemplo conhecido, não prova independente.

**Decisões.** Nenhum modelo aprendido foi aprovado para interpretação automática.
Preservaram-se CLI local, recuperação, citações e abstenção. A integração anterior
do extrator Rust é distinta: o provedor GitHub importa `reports::analyze` e filtra
o estado desligado em `report_for`; isso não é implantação dos modelos do laboratório.

**Artefatos.** [Avaliação inicial](FRX-AI-AVALIACAO.md),
[relatório P1](FRX-AI-RND-2026-10-29.md),
[README v1](../research/frx_ai/README.md),
[resultado P1](../research/frx_ai/snapshot/final-results.json),
[extrator Rust](../src-tauri/src/fixes/reports.rs),
[uso no provedor](../src-tauri/src/fixes/providers/github.rs).

<a id="lab-v2"></a>
### FRX AI v2 — relações, contraste e recuperação especializada

**Objetivo.** Separar campos da relação técnica e testar treinamento real de
encoders, com calibração e avaliação em jogos novos.

**Hipóteses.** Spans, múltiplas cabeças, contraste e relações por objetivo
melhorariam interpretação; recuperação poderia melhorar separadamente.

**Fundamentação científica.** DocRED, PURE, ERICA, SetFit, supervisão fraca,
active learning, temperature scaling e classificação seletiva; adaptações de
princípios, sem reprodução integral desses sistemas.

**Metodologia.** Treino ampliado de 167 relações/13 jogos; dev 42/quatro jogos;
calibração 18/um jogo. E07 usou 113 reais e 320 sintéticas. A tarefa E18 por
ação/observação tinha divisões próprias de 183/44/20. P2-R foi reservado por jogo,
selado antes da anotação e avaliado sem retreino; teste v1 passou a material conhecido.

**Implementação.** MiniLM efetivamente ajustado com cabeças de resultado,
direção, modalidade e agrupamento, perda multitarefa e contraste supervisionado.
E18 separa objetivos. Um dual encoder foi treinado para recuperação. Propostas
e validações de contexto continuaram determinísticas; exportação ONNX/INT8.

**Experimentos.** 42 configurações com parâmetros aprendidos: 18 neurais,
12 TF-IDF/logística, dez simulações de aquisição ativa e dois treinos de
recuperação. E09 com spans de referência não participou da seleção. Não são
42 arquiteturas neurais distintas.

**Resultados.** Em P2-R, E07 e E18 acertaram integralmente quatro campos em
14/96 casos cada. E07: macro F1 de resultado 0,286, 1/21 claims brutos corretos
no limiar 0,7, sete falsos sucessos; E18: 0,237, 1/4 e três falsos sucessos.
Após calibração, ambos aceitaram zero claims. Reduzir ECE de 0,303 a 0,072
não tornou a interpretação útil.

| P2-I, 39 consultas respondíveis | P@3 | NDCG@5 | MRR |
|---|---:|---:|---:|
| MiniLM original FP32 | 0,513 | 0,658 | 0,731 |
| MiniLM original INT8, controle posterior | 0,487 | 0,659 | 0,740 |
| FRX contrastivo INT8 selecionado | 0,504 | 0,720 | 0,823 |

O topo foi relevante em 28/39 consultas contra 22/39 do original FP32; P@3 caiu
nessa comparação. No controle INT8 pareado posterior, o intervalo por jogo de
ganho de NDCG foi [0,039; 0,097]; P@3 permaneceu inconclusivo.

**Descobertas.** Representações especializadas ajudaram a ordenação, enquanto
perda menor de treino, mais sintéticos e melhores scores não garantiram
generalização das relações. O treino E07 melhorou até a sexta época; menor
perda na sétima coincidiu com pior F1 de desenvolvimento.

**Falhas e limitações.** Só propostas determinísticas foram avaliadas; não há
recall global de entidades. Rótulos provisórios, consultas por templates,
poucos jogos e exclusão das consultas sem relevante das médias de recuperação.
Conformal não certifica precisão sob mudança de jogo e calibração pequena.

**Decisões.** Preservar o recuperador treinado e desativar aceitação automática
da interpretação. Não generalizar a eficácia de correções a partir do ranking.

**Artefatos.** [Relatório v2](FRX-AI-SCIENTIFIC-2026-10-29.md),
[README](../research/frx_ai/v2/README.md),
[experimentos](../research/frx_ai/v2/snapshot/experiments.json),
[interpretação](../research/frx_ai/v2/snapshot/final-results-v2.json),
[recuperação nativa](../research/frx_ai/v2/snapshot/retrieval-final-v2.json),
[controle INT8](../research/frx_ai/v2/snapshot/matched-quantization-control.json).

<a id="lab-v3"></a>
### FRX AI v3 — investigação, memória e dependências probabilísticas

**Objetivo.** Ampliar a avaliação para diagnóstico estruturado, memória de
tentativas e seleção de informações, mantendo o recuperador v2.

**Hipóteses.** Cabeças separadas e reranking poderiam ajudar; restrições e
histórico evitariam conclusões indevidas; ganho de informação dependeria da
qualidade do modelo de observações e de suas dependências.

**Fundamentação científica.** Extração de eventos/documentos, AutoTSG, SuperDRI,
StepFly e troubleshooting bayesiano. SafeOpt foi investigado e adiado por falta
de função de segurança e recompensas reais justificáveis.

**Metodologia.** 209 relações antigas no treino; antigo teste v2 tornou-se dev
conhecido. P3 reservou 95 comentários em seis discussões, antes de ler os corpos,
sem seleção por palavras de sucesso. Simulações ficaram explicitamente
separadas de relatos reais. A montagem final assistida é posterior ao teste do challenger.

**Implementação.** Pesos neurais v2 preservados, cabeças lineares, rankers
pairwise, likelihoods com suavização e árvore por informação mútua condicional.
Observações literais de logs, restrições de três estados, grafo pequeno em memória
e histórico persistente são engenharia determinística, sem executor de correções.

**Experimentos.** 24 configurações aprendidas: 12 cabeças de eventos, seis
rankers, três modelos de likelihood/contexto e três de dependência. Não houve
novo fine-tuning de encoder.

**Resultados.** Modalidade em P3: macro F1 0,310, 25/63 seleções corretas e
33 falsas classificações de execução. Observação: macro F1 0,204, zero claims
aceitos. Na recuperação, FRX v2 preservado obteve NDCG@5 0,794 contra 0,725 do
MiniLM original INT8; MRR 0,945/0,919 e P@3 0,634/0,591. Eram 31 consultas
respondíveis e 17 sem relevante. O challenger por janelas ficou em NDCG 0,772.

Na simulação final de 600 incidentes, Bayes ingênuo/ordem fixa, selecionado no dev,
teve 84,3% de acurácia; com ganho de informação, 82,5% e 52 diagnósticos errados
confiantes, contra 31 na ordem fixa. Agrupar duplicatas atingiu 85,2%, sem adoção
retrospectiva. Sob mudança de ruído, a política selecionada caiu a 56,8%.

**Descobertas.** O ganho do recuperador anterior resistiu a outra coorte:
intervalo de ganho de NDCG [0,028; 0,124], em seis clusters. Contar observações
dependentes como independentes pode aumentar confiança errada. Uma árvore mais
complexa não demonstrou vantagem necessária.

**Falhas e limitações.** Propostas ruins, metáforas, templates, atribuição a outro
objetivo e simulação com hipóteses explícitas. As oito demonstrações históricas
de logs foram selecionadas pelo parser; não estimam precisão diagnóstica real.
Compatibilidade por strings não é compatibilidade aprendida de hardware.

**Decisões.** Manter FRX v2, memória e investigação com causas desconhecidas;
omitir cabeças de interpretação por padrão. Não transferir probabilidades
simuladas a jogos reais nem declarar validado o fluxo combinado posterior.

**Artefatos.** [Relatório v3](FRX-AI-MULTIDISCIPLINARY-2026-10-30.md),
[README](../research/frx_ai/v3/README.md),
[experimentos](../research/frx_ai/v3/snapshot/experiments.json),
[avaliação](../research/frx_ai/v3/snapshot/final-results.json),
[memória](../research/frx_ai/v3/memory.py),
[simulação de estrutura](../research/frx_ai/v3/snapshot/structure-final.json).

<a id="lab-v4"></a>
### FRX AI v4 — gatilhos de eventos e verificação conservadora

**Objetivo.** Localizar eventos e verificar suas unidades antes de reconstruir
procedimentos, reduzindo falsas ações executadas.

**Hipóteses.** Treinar gatilhos/estado/direção e exigir evidência literal poderia
melhorar precisão; rótulos auditados e contraste poderiam ajudar a extração.

**Fundamentação científica.** Extração de eventos em etapas, diretrizes de
anotação e Confident Learning. A auditoria usa sinais estruturais; não foi
implementada correção automática de rótulos por cleanlab ou concordância com modelo.

**Metodologia.** Auditoria de 305 registros antigos, sem sobrescrita dos rótulos.
Treino: 45 parágrafos/58 eventos/sete jogos; dev: 24/33/oito jogos, material
conhecido; 160 contrastes sintéticos só no treino. P4 reservado e congelado
antes da inspeção final, com cinco AppIDs novos e grupos desiguais.

**Implementação.** MiniLM-L6-v2 com cabeças por token; ajuste das duas últimas
camadas em parte dos modelos. Verificador literal determinístico, recuperação
v2 e diagnóstico/memória v3 reutilizados. Detector exportado em ONNX float.

**Experimentos.** Seis treinos S1–S6, oito épocas cada; 34 leituras de dev e
dois controles de exportação não foram outros 36 treinos. S3 foi selecionado
antes do final. S5/S6 não isolaram vantagem relevante da perda contrastiva.

**Resultados.** Em P4, sem verificador: 14/41 execuções corretas, 27 falsas.
Com verificador selecionado: 5/5, zero falsas, recall 5/33 (15,2%). Lexical com
o mesmo verificador: 7/8, uma falsa, recall 21,2%. Tuplas estado+direção aceitas:
8/16 corretas. Zero relações recuperadas entre 20 observações de referência,
inclusive no controle fornecendo eventos/estados corretos.

**Descobertas.** Rejeição conservadora reduz erros e pode eliminar utilidade.
Cinco previsões corretas não demonstram precisão geral ≥90%: Wilson descritivo
aproximado de 56,6–100%, abaixo dos pisos amostrais do próprio protocolo.
Evidência literal íntegra não garante semântica correta.

**Falhas e limitações.** Cobertura baixa, direção errada, parser estreito de
resultados, coreferência não demonstrada. Controles com gatilhos corretos não
são extração ponta a ponta. Os 12 controles de próxima verificação deram
12/12 no v4 e 11/12 no v3, mas são casos históricos selecionados.

**Decisões.** Relações desabilitadas por padrão; preservar verificação e memória,
investigar argumentos e observações com mais cobertura. Uma guarda posterior
em `run.py` corrigiu tentativas fornecidas com contexto incompatível; esse
conserto de isolamento não alterou o resultado de linguagem congelado.

**Artefatos.** [README v4](../research/frx_ai/v4/README.md),
[relatório](../research/frx_ai/v4/REPORT.md),
[experimentos](../research/frx_ai/v4/snapshot/experiments.json),
[resultado](../research/frx_ai/v4/snapshot/final-results.json),
[anotação](../research/frx_ai/v4/ANNOTATION.md).

<a id="lab-v5"></a>
### FRX AI v5 — decomposição sintática e incidentes

**Objetivo.** Recuperar mais tentativas e relações com evidência, sem depender
de rejeição quase total, e testar memória/replay de incidentes.

**Hipóteses.** Sintaxe pré-treinada e decomposição poderiam separar negação,
modalidade e resultado; afirmações atômicas e grupos completos ajudariam os verificadores.

**Fundamentação científica.** DrDEE inspirou decomposição/reconstrução;
SuperDRI e Cloud-OpsBench motivaram incidentes tipados e trajetórias reprodutíveis.
Esses sistemas não foram reproduzidos integralmente.

**Metodologia.** Treino conhecido de 69 unidades/91 eventos, mais 14 unidades
auditadas/36 eventos. Antigo final v4 virou dev de 57 unidades/64 eventos.
P5 foi reservado e anotado depois do congelamento; rótulos permanecem do agente.
O AppID da discussão não assegura que todos os acontecimentos sejam desse jogo.

**Implementação.** Stanza pré-treinado na seleção, spaCy no controle; reconstrução
determinística por gatilho, argumentos, modalidade e observações. MiniLM ajustado
para quatro afirmações binárias; logística/árvore para associações. Os novos
verificadores treinados não foram adotados na seleção final.

**Experimentos.** 18 ajustes exploratórios (seis MiniLM, 12 de associação) e
dois controles MiniLM de reprodução: 20 execuções de treino, não 20 arquiteturas.
Resumo de incidentes, comentário completo e híbrido foram controles de recuperação.

**Resultados.** Comparação pareada em P5:

| Pipeline | Execuções corretas/afirmadas | Recall de 52 | Falsas execuções | Relações corretas/propostas | Falsos sucessos |
|---|---:|---:|---:|---:|---:|
| v4 conservador reavaliado | 14/14 | 26,9% | 0 | 1/1 | 0 |
| v5 selecionado | 28/32 | 53,8% | 4 | 6/9 | 1 |

Recall global de relações: 6/32. Duas das seis relações linguisticamente corretas
pertenciam a outros jogos; somente quatro eram do jogo da discussão. Excluindo
as duas execuções estrangeiras, a auditoria reporta 26/30, recall 26/50.
Dois dos três grupos conjuntos propostos corresponderam à referência, sem
eficácia causal individual ou validação no computador atual.

**Descobertas.** Cobertura cresceu, mas configuração anterior/condicional ainda
virou tentativa. Resumir incidentes perdeu informação: no replay conhecido v3,
NDCG 0,668 no resumo, 0,767 híbrido e 0,794 no recuperador/comentários preservados.
Nove cenários/dez passos de replay deram 10/10 próximas verificações, sem
demonstrar diagnóstico aprendido.

**Falhas e limitações.** A5 registrava 91 eventos onde se esperavam 127 e não
guardava hash dos inputs para certificar a composição. O parser com cache frio
consumia RNG do Torch, confundindo A1–A5. Uma seed igual no começo não isolava
arquitetura/dados. A6 restabelece seeds depois dos atributos e registra hashes.

**Decisões.** Manter recuperador v2 e inferência selecionada reprodutível,
verificadores aprendidos desativados. Depois da auditoria, a entrada pública
`run.py` deixa atribuição de jogo por evento não verificada; esse bloqueio não
conta como resolução científica de coreferência. Próxima etapa: identidade do evento.

**Artefatos.** [README v5](../research/frx_ai/v5/README.md),
[relatório](../research/frx_ai/v5/REPORT.md),
[experimentos](../research/frx_ai/v5/snapshot/experiments.json),
[resultado](../research/frx_ai/v5/snapshot/final-results.json),
[auditoria de jogos](../research/frx_ai/v5/snapshot/posthoc-review.json),
[RNG](../research/frx_ai/v5/snapshot/seed-audit.json).

<a id="lab-v6"></a>
### FRX AI v6 — identidade contextual e gargalo de candidatos

**Objetivo.** Associar ações/resultados ao evento, jogo e ator corretos,
preservando cobertura e investigando o custo da inferência.

**Hipóteses.** Identidade por evento, geração de grupos seguida de verificação,
afirmações atômicas e memória exata poderiam reduzir atribuições indevidas.

**Fundamentação científica.** EPAL, extração de relações em duas etapas,
DrDEE, coreferência, claim verification e benchmarks de investigação. As regras
de identidade implementadas não são um modelo aprendido de coreferência.

**Metodologia.** 83 unidades antigas de treino/auditoria, sem o teste v5: 133
afirmações sobre texto real, incluindo claims construídos, e 144 sobre 36 textos
sintéticos. Dev novo: 48 trechos/seis jogos. P6-M/P6-C reservados antes da seleção;
rótulos travados antes das previsões. Fontes/citações repetidas não são independentes.

**Implementação.** FRX v2 preservado, sintaxe compacta spaCy, evidências com
offsets e identidades desconhecidas explícitas. Seleção: logística multiclasse
L1 de 19 atributos/2.141 bytes, score não calibrado e limiar 0,3. Regras de
escopo, grupos e logs permanecem determinísticas. Comparadores incluem
cross-encoders, NLI e Stanza v5; nenhum modelo diagnóstico foi treinado.

**Experimentos.** Seis configurações treinadas (três lineares e três
cross-encoders), mais dois refits de reprodução. N4 não foi executado. No dev,
L1 teve 11/11 relações corretas de 21 anotadas; neurais ficaram em 9/15, 10/15
e 11/15, com três, dois e um falsos sucessos. NLI só propôs uma relação correta.

**Resultados.** Comparação pareada em P6-M, sem reutilizar os números históricos de P5:

| Medida | v5 Stanza reavaliado | v6 selecionado |
|---|---:|---:|
| Execuções corretas/afirmadas | 12/12 | 19/23 |
| Recall de 43 execuções | 27,9% | 44,2% |
| Falsas execuções estritas | 0 | 4 |
| Relações corretas/propostas | 4/4 | 7/9 |
| Recall de 48 relações | 8,3% | 14,6% |
| Atribuições incorretas de sucesso | 0 | 2 |

P6-C: 9/15 execuções corretas e 2/3 relações, com seis falsas execuções e um
falso sucesso. Agregado descritivo: **28/38 execuções corretas**, dez falsas;
**9/12 relações corretas**, três falsos sucessos e **61/70 não recuperadas**.
Nenhuma das sete relações conjuntas foi recuperada. Não há melhoria segura
de interpretação demonstrada; o intervalo de ganho de recall por jogo no
principal, [0; 12,3] pontos percentuais, inclui zero.

**Descobertas.** **Somente 11/70 relações verdadeiras chegaram completas ao
verificador.** Com esse gerador fixo, mesmo um classificador perfeito não
ultrapassa 15,7% de recall. Entre as 61 perdidas, 14 envolvem estados
preexistentes/condicionais excluídos pelo gerador; 47 têm proprietários executados
e perdas de spans, observações ou agrupamento. Melhorar só a classificação não basta.

**Falhas e limitações.** Negação ainda virou sucesso e grupos incompletos
receberam crédito individual. Dez falsas execuções estritas incluem erros
semânticos, duplicatas, diferenças de cabeça verbal e omissões dos rótulos;
não foram reclassificadas após olhar o teste. Houve 4/5 atribuições de AppID e
34/39 contas de ator corretas, entre 11 e 55 identidades anotadas, respectivamente;
poucos pares reais de jogos distintos impedem
concluir que coreferência foi resolvida. Temporalidade não foi anotada exaustivamente.

**Decisões.** Manter tudo experimental, sem conexão ao Fix Engine, e preservar
o recuperador v2. Uma falha de intervalo invertido abortou a primeira execução
antes de métricas; a correção mínima posterior ao selo está registrada, sem
retunar pesos/limiares/rótulos. Próxima hipótese: eventos canônicos e candidatos
completos antes de polaridade, com revisão independente e nova aceitação.

**Artefatos.** [README v6](../research/frx_ai/v6/README.md),
[relatório](../research/frx_ai/v6/REPORT.md),
[experimentos](../research/frx_ai/v6/snapshot/experiments.json),
[resultado](../research/frx_ai/v6/snapshot/final-results.json),
[auditoria](../research/frx_ai/v6/snapshot/posthoc-metrics.json),
[gerador](../research/frx_ai/v6/pairs.py),
[avaliador](../research/frx_ai/v6/evaluate.py),
[reparo de offsets](../research/frx_ai/v6/snapshot/offset-repair.json).

**Aplicação dos achados (30/10/2026).** As descobertas D-002 (recuperação
especializada), D-008 (execução ≠ sugestão ≠ resultado) e D-010 (candidatos
completos limitam o classificador) fundamentaram o escopo do módulo de busca do
aplicativo (`docs/AI-SEARCH.md`). O módulo usa apenas o recuperador e não
emprega nenhum modelo de interpretação. A pesquisa de interpretação continua
experimental e desconectada do Fix Engine; nada em `ai/` reabre v1–v6.

<a id="lab-v7"></a>
### FRX AI v7 — descoberta de candidatos e raciocínio diagnóstico

**Objetivo.** Localizar as relações perdidas em v6, reduzir o gargalo de candidatos,
representar procedimentos e construir um primeiro ciclo de investigação baseado em evidências.

**Hipóteses.** As perdas estão antes do verificador; com candidatos corretos ele recupera muito
mais; separar proposta de decisão eleva a cobertura; representar o procedimento recupera conjuntos;
seleção aprendida da próxima verificação supera baselines.

**Fundamentação científica.** Filtragem em dois estágios, recuperação de candidatos a perguntas,
dependências entre passos, contra-afirmações causais e supervisão por funções de rótulo. Nenhum
sistema foi reproduzido. Os Termos Comerciais da Anthropic motivaram não treinar com saída do agente
(corrigido na [errata E-001](#errata-e-001)).

**Metodologia.** [Protocolo](../research/frx_ai/v7/PROTOCOL.md) registrado antes dos dados novos.
Instantâneo público de 9.900 issues e 30.000 comentários. P7, J7 e C7 reservados sem exibição; código
congelado por hash; anotação; travamento; uma previsão. Avaliador de v6 sem alterações. Uma emenda de
infraestrutura antes de qualquer métrica. Critério de sucesso fixado antes do teste.

**Implementação.** Gerador com modalidade como atributo, resultados por oração, ações ampliadas e
estrutura `single`/`joint`/`sequence`/`alternatives`, por regras sobre parser pré-treinado. Investigador
com sinais tipados, hipóteses com a favor/contra, verificações, histórico por ambiente, busca local com o
recuperador v2 e CLI. Tudo explícito; nenhum modelo de diagnóstico treinado é usado por padrão.

**Experimentos.** Atribuição de perdas, candidatos-oráculo, gerador isolado, procedimentos conjuntos,
comparação em P7/J7, 12 configurações de política em D7 (validação cruzada e uma leitura de teste, sem
novo encoder), cinco políticas em C7. Nenhuma rede neural foi treinada.

**Resultados.** Das 70 relações de P6: 59 perdidas antes do verificador (modalidade 25, evento 13,
resultado 11, fronteira 9, classe 1, formação de par 0), duas rejeitadas, nove recuperadas. Com
candidatos corretos, o mesmo L1 fez 41/70 com filtro e 57/70 sem filtro.

| Teste novo | v6 selecionado | v7 |
|---|---:|---:|
| P7-M, candidatos completos | 4/30 | 13/30 |
| P7-M, relações corretas/propostas | 2/3 | 9/16 |
| P7-M, recall; falsos sucessos | 6,7%; 1 | 30,0%; 3 |
| J7, candidatos completos | 10/67 | 28/67 |
| J7, relações corretas/propostas | 8/27 | 14/32 |
| J7, recall; falsos sucessos | 11,9%; 6 | 20,9%; 5 |
| J7, grupos propostos/recuperados de 20 | 3/3 | 6/2 |

D7, teste com 115 pedidos, top-1: maioria 58,3%, logística TF-IDF + estado 57,4%, recuperação de
casos selecionada 54,8%, embeddings 39,1%, regras fixas 26,1%. Ganho do selecionado sobre a maioria
por grupo: [−8,8; +0,9] pontos. C7, top-1: regras v7 8/24, sempre log 7/24, híbrido 4/24 pelos marcos
do agente; pelo mantenedor (nove casos), sempre log 6/9, regras v7 3/9.

**Descobertas.** Ver [D-013](#d-013) a [D-018](#d-018).

**Falhas e limitações.** Rótulos e gerador do mesmo agente, sem revisão humana. Ganho de recall de
relações com intervalo incluindo zero. Precisão de 56% em P7-M, abaixo da meta histórica. Falsos
sucessos absolutos subiram em P7-M. Crédito individual indevido persiste. Template de issue contaminou
sintomas e só foi visto depois do teste de C7. Logs anexados não são lidos. Recuperação sem detecção de
ausência de resposta. Português não avaliado.

**Decisões.** Publicar v7 como laboratório, sem conexão com o Fix Engine. Regras como política padrão
da CLI. Não adotar seletor treinado. Não treinar com saída do agente; a decisão sobre modelos anteriores
fica com o responsável ([registrada na seção XIII](#proveniencia)). Correções pós-teste só como replay conhecido.

**Artefatos.** [README v7](../research/frx_ai/v7/README.md),
[relatório](../research/frx_ai/v7/REPORT.md),
[literatura](../research/frx_ai/v7/LITERATURE.md),
[resultados](../research/frx_ai/v7/snapshot/final-results.json),
[perdas em P6](../research/frx_ai/v7/snapshot/waterfall-p6.json),
[oráculo](../research/frx_ai/v7/snapshot/oracle-p6.json),
[D7](../research/frx_ai/v7/snapshot/selector-final.json),
[C7](../research/frx_ai/v7/snapshot/c7-results.json),
[emendas](../research/frx_ai/v7/snapshot/amendments.json).

<a id="lab-v8"></a>
### FRX AI v8 — resultados, procedimentos e trajetórias sem novo treino

**Objetivo.** Aproximar a FRX AI de investigar problemas reais, medindo sozinhas a detecção de resultados, a representação de
procedimentos conjuntos e trajetórias completas de diagnóstico.

**Hipóteses.** Separar estado de resultado reduz falsos resultados sem perder cobertura; pesos congelados não superam regras;
o problema dos grupos é de representação; escolher a verificação que separa hipóteses supera baselines; regras de log superam
busca literal.

**Fundamento científico.** Decorre de D-013, D-016 e D-020, sem nova literatura. O guia de anotação define a tarefa.

**Metodologia.** Protocolo antes dos dados; coorte de desenvolvimento; código congelado por hash; sorteio de teste depois do
congelamento; rótulos H0 travados antes das previsões; leitura única. Threads de v1–v7 e do piloto excluídos.

**Implementação.** `research/frx_ai/v8/`: detector, grupos, estado do caso, replay, logs com redação e bloqueio de caminhos,
CLI e simulador. **Nada aprendido.** Comparadores: v7, regex do v2 e pesos E07 congelados.

**Experimentos.** Coortes O8, T8 e L8, e análise de sensibilidade posterior, identificada como tal.

**Resultados (H0, não é validação humana).**

| Medida | v8 | Comparador |
|---|---:|---:|
| O8, afirmações achadas | 22/32 | v7 28/32 |
| O8, falsos positivos | 3 de 15 | v7 6 de 13 |
| O8, classe, com positivo fundido | 37/58 | v7 22/58; E07 sem filtro 32/58 |
| O8, diferença de classe | | contra v7 +25,9 pontos [10,3; 41,4]; contra E07 sem filtro +8,6 [−6,9; 24,1] |
| O8, grupos corretos | 1/14 | v7 0/14 |
| T8, top-3 | 25/26 | sempre-log 8/26; regras v7 10/26 |
| T8, top-1 | 21/26 | sempre-log 7/26; regras v7 2/26 |
| T8, redundante | 1/26 | sempre-log 14/26; regras v7 12/26 |
| T8, decisão mudou quando devia | 6/12 | sempre-log 4/12; regras v7 5/12 |
| T8, resultados da conversa achados | 7/18, com 31 extraídos | n/a |
| L8, trechos relevantes | 3/7 | busca literal 6/7 |

Sensibilidade posterior: ao tratar pedir e ler o log como a mesma ação, o top-1 fica em 21/26 contra 18/26 de sempre-log
(+11,5 pontos [0,0; 29,2]).

**Descobertas.** Ver [D-021](#d-021) a [D-024](#d-024).

**Falhas e limites.** Rótulos e regras têm o mesmo autor. Cobertura abaixo do critério. A cobertura caiu de 16/17 no
desenvolvimento para 22/32 no teste. Amostras pequenas, quase só em inglês. A política recomenda ler anexos que o protótipo não lê.
Causa sem suporte igual a 0 vem da construção do sistema. Nada foi testado em jogo real.

**Decisões.** Nenhum treino. Sem adicionar mais regex antes do gabarito humano. Nada entra no aplicativo.

**Artefatos.** [Relatório](../research/frx_ai/v8/REPORT.md), [protocolo](../research/frx_ai/v8/PROTOCOL.md),
[snapshot](../research/frx_ai/v8/snapshot/first-reading.json), [piloto](../research/frx_ai/benchmark/PILOT-PROTOCOL.md).

<a id="lab-v9"></a>
### FRX AI v9 — pesquisa técnica online

**Objetivo.** Testar se a investigação online iterativa, orientada pelo contexto técnico, produz diagnósticos mais úteis que conhecimento local ou uma pesquisa única.

**Hipóteses.** H9: seguir pistas supera pesquisa única e consultas fixas, com orçamento igual. A condição favorável exigia IC pareado acima de 0.

**Fundamento científico.** ReAct e IRCoT inspiraram a estrutura ação → observação → decisão, mas sem LLM. Nenhum foi reproduzido. A orientação da OWASP motivou limites fora de qualquer decisão.

**Metodologia.** Seis condições com o mesmo leitor: sem internet, pesquisa única, única técnica, consultas fixas, pista sorteada e iterativa; mais uma com MiniLM congelado. Protocolo prévio, código congelado, sorteio posterior, gabarito H0 travado e leitura única.

**Implementação.** `research/frx_ai/v9/`. Regras: interpretação, roteamento, formulação de consultas, pistas, aplicabilidade, parada, hipóteses e verificações. Aprendido: só o MiniLM público congelado, como reordenador opcional. **O Claude não é chamado em execução.** Sem modelo gerativo local nesta máquina.

**Experimentos.** Coortes R9 e R9-L, continuidade em quatro turnos, 31 testes de segurança e replays de diagnóstico posteriores.

**Resultados (H0, não é validação humana).**

| Medida | C iterativa | Comparador |
|---|---:|---:|
| Conceito no relatório | 1/17 | B 1/17; B2 1/17; F 0/17; A 0/17 |
| C − B | | 0,0 [0,0; 0,0] |
| Conceito lido | 4/17 | B 3/17; B2 3/17; F 2/17 |
| Próxima verificação, top-3 | 17/24 | A 19/24; F 19/24; B 17/24 |
| Documentos relevantes, só pelo título | 26/130 | B 20/62 |
| Consultas posteriores com conceito novo | 1/30 | n/a |
| Citações literais com hash correto | 57/57 | n/a |
| Replay P2, oráculo: lido e no relatório | 16/17 e 1/17 | diagnóstico posterior |
| Ao vivo, 5 casos: conceito no relatório | 1/5 | B 0/5; F 0/5 |

**Descobertas.** Ver [D-025](#d-025) a [D-028](#d-028).

**Falhas e limites.** H9 não sustentada. Em 5 de 9 relatórios provisórios, as pistas enganavam. Autor único de regras, gabarito e julgamentos. Só um repositório na prática. Na primeira rodada ao vivo, o executor insistiu após um limite secundário do GitHub: 13 execuções foram descartadas e o código foi corrigido.

**Decisões.** Nenhum treino. Nada entra no aplicativo. A prioridade passa a ser o leitor de evidências, e não novos conectores.

**Artefatos.** [Relatório](../research/frx_ai/v9/REPORT.md), [protocolo](../research/frx_ai/v9/PROTOCOL.md), [leitura congelada](../research/frx_ai/v9/snapshot/first-reading.json), [replays](../research/frx_ai/v9/snapshot/replays.json).

<a id="linha-do-tempo"></a>
## V. Linha do tempo científica

Datas são as dos registros locais, não estimativas de duração da pesquisa.

| Etapa | Pergunta científica | Descoberta e decisão | Próximo obstáculo |
|---|---|---|---|
| v1 e antecedentes — 29/10/2026 | Classificação/LLM/NLI interpretam relatos técnicos? | Resultados pequenos das regras não generalizaram como garantia; híbridos aprendidos falharam | Unidade de anotação, direção, negação e relações |
| v2 — 29/10/2026 | Fine-tuning e contraste ajudam compreensão e busca? | Recuperação treinada melhorou ordenação; interpretação/calibração não tiveram utilidade suficiente | Propostas, objetivos e relações completas |
| v3 — 30/10/2026 | Como investigar usando contexto, logs e histórico? | Recuperador v2 resistiu a coorte nova; dependência/ruído limitaram Bayes simulado | Diagnóstico real e eventos confiáveis |
| v4 — 30/10/2026 | Eventos em etapas evitam falsas execuções? | Verificação reduziu erros sacrificando recall; relações não foram recuperadas | Argumentos, observações e cobertura |
| v5 — 30/10/2026 | Sintaxe recupera tentativas/procedimentos? | Mais cobertura e algumas relações; falso sucesso e eventos de outros jogos | Identidade e reprodução sem confusão de RNG |
| v6 — 30/10/2026 | Identidade/verificação contextual corrigem as associações? | Protótipo menor; candidatos completos limitados a 11/70; segurança semântica ainda insuficiente | Representação canônica, agrupamento e avaliação revisada |
| v7 — 30/10/2026 | Onde se perdem as relações, e uma política aprendida escolhe a próxima verificação? | Verificador não era o gargalo; cobertura cresceu em dados novos; grupos não melhoraram; demonstrações ensinam triagem | Detecção de resultados, grupos, rótulos de utilidade e leitura real de logs |
| v8 — 30/10/2026 | Medidas sozinhas, regras explicam resultados, grupos e trajetórias? | Menos falsos resultados, menor cobertura; grupos falham na extração; vantagem vem de notar logs já fornecidos. Sem treino | Gabarito humano, acompanhamento da conversa e leitura de anexos |
| v9 — 31/10/2026 | Pesquisar online e seguir pistas melhora a investigação? | Pesquisa real e rastreável, mas iterativa empatou com pesquisa única; a resposta é lida e não chega ao relatório. Sem treino | Leitor de evidências |

<a id="estado-atual"></a>
## VI. Estado da pesquisa após o v7

| Capacidade | Evidência favorável | Estado permitido |
|---|---|---|
| Recuperação especializada | P2-I e nova coorte P3 favoreceram o recuperador v2 em NDCG; replays v4/v5 o preservaram | Componente promissor de busca; relevância não confirma eficácia |
| Identificação de alguns eventos | Comparações pareadas P5/P6-M recuperaram mais execuções | Experimental; falsos positivos, identidade e cobertura persistem |
| Memória contextual | Contratos e processos separados demonstraram persistência/isolamento de tentativas do chamador | Ferramenta experimental; não aprende causalidade |
| Investigação com logs explícitos | Perguntas pertinentes em exemplos históricos e replays limitados | Assistência por regras; causa desconhecida permanece desconhecida |
| Inferência local compacta | v6 sem Torch, artefatos ativos ≈38,7 MB; 23 contratos e CLI com 11 casos | Viabilidade técnica; não aprovação linguística ou de produção |
| Descoberta de candidatos | P7-M 13/30 contra 4/30; J7 28/67 contra 10/67, mesmo protocolo | Experimental; ganho robusto só em J7; decisão ainda erra |
| Ciclo de investigação | C7: citações 24/24, sem causa confirmada 24/24, segurança 24/24; próxima verificação sem vantagem clara | Protótipo por regras; não é diagnóstico aprendido nem validado em jogo real |

**Continuam sem confiabilidade demonstrada:** compreensão completa de
procedimentos, atribuição correta de todos os resultados, reconstrução de
ações conjuntas em jogos novos, diagnóstico autônomo real, seleção aprendida
de correções e aplicação autônoma segura. Nenhum resultado da pesquisa autoriza
instalações. Não há revisão humana independente documentada em v1–v6.

O benchmark pareado v6 usou 12 inputs conhecidos, caches de análise separados
e processos novos: cerca de 222 MiB RSS, mediana aquecida 14,7 ms e P95 27,5 ms;
o v5 nesse ensaio teve 812 MiB, 64,5 ms e 218 ms. Isso é distinto do benchmark
histórico v5 de 797 MiB/142 ms. Os pipelines têm comportamentos e cobertura
diferentes; redução de custo não demonstra melhor compreensão. Python,
dependências, rede e instalação completa não estão no tamanho dos artefatos.

Replay diagnóstico conhecido v6: 10/10 próximas verificações contra 8/10 no v5;
três probes posteriores deram 3/3 em ambos. Os templates derivam de casos
conhecidos: não há precisão diagnóstica independente nem diagnóstico aprendido.

Em casos novos (C7), essas mesmas regras de v6 deram 7/24 pelos marcos do agente, sempre
escolhendo coletar o log: o 10/10 anterior não se manteve. Custo do v7, em outra coorte, portanto sem
comparação direta com v6: extração com parse em cache 2,6 ms (P95 8,0 ms) e 340 MiB; CLI completa
4,2 s e 731 MiB; CLI só com regras 0,06 s e 20 MiB. Índice local de evidências: 63 MB.

### Atualização após o v8

Rótulos H0 do agente. Não é validação humana independente.

| Capacidade | Origem | Evidência | Estado permitido |
|---|---|---|---|
| Separar estado de resultado | Regra | O8: 3 falsos positivos contra 6; cobertura 22/32 contra 28/32 | Experimental; cobertura abaixo do critério |
| Representar grupos sem crédito individual | Regra | O8: 0 créditos falsos, 1/14 grupos corretos | Representação serve; extração falha |
| Não pedir o que já foi fornecido | Regra | T8: 1/26 redundantes contra 12/26 e 14/26 | Regra útil; não é diagnóstico inteligente |
| Separar hipóteses na próxima verificação | Regra | T8: 21/26 contra 18/26 ao fundir pedir e ler log, IC [0,0; 29,2] pontos | Vantagem frágil, sem replicação |
| Acompanhar uma conversa | Regra | T8: 7/18 resultados, 31 extraídos | Não confiável |
| Recuperar evidências | Recuperação; R02 aprendido e congelado | Sem medida nova | Conforme as linhas acima |
| Seleção aprendida de verificações | Aprendido | Nenhuma vantagem consistente demonstrada | Não anunciar |

### Atualização após o v9

Rótulos H0 do agente. Não é validação humana independente.

| Capacidade | Origem | Evidência | Estado permitido |
|---|---|---|---|
| Consultar fontes públicas com segurança | Regra | Ao vivo sem credenciais; 31 testes | Ferramenta funcional e limitada; não é inteligência |
| Trilha rastreável | Regra | 57/57 citações literais com hash | Demonstrado em dados congelados |
| Seguir pistas | Regra | 1/30 consultas posteriores com conceito novo; empate com pesquisa única | Sem vantagem demonstrada |
| Atualizar o caso entre turnos | Regra | 4 sequências preparadas | Promissor; não é diagnóstico |
| Entender evidências | Regra | Oráculo: 16/17 lidos, 1/17 no relatório | **Gargalo; não funciona** |
| Reordenar por semelhança | MiniLM congelado | S − C 0,0 [−17,6; 17,6] pontos | Sem ganho demonstrado |
| Planejar com modelo local | n/a | Não testado | Sem evidência |

<a id="descobertas"></a>
## VII. Descobertas científicas acumuladas

IDs `D-001` em diante são permanentes. “Descoberta” aqui é conhecimento do
laboratório com limites, não lei universal sobre modelos ou jogos.

<a id="d-001"></a>
### D-001 — Classificar o comentário inteiro perde a relação específica

- **Origem/hipótese:** antecedentes/v1; sentimento global representaria o efeito de cada opção.
- **Evidência:** relatos com ações diferentes e efeitos opostos, falsos sucessos em P0/P1.
- **Conclusão permitida:** a representação global testada foi inadequada para esses casos.
- **Limites:** não prova impossibilidade de compreensão documental por outro modelo.
- **Consequência:** separar evento, alvo, objetivo, resultado e evidência.
- **Referência:** [avaliação inicial](FRX-AI-AVALIACAO.md), [v1](#lab-v1).

<a id="d-002"></a>
### D-002 — Recuperação pode avançar sem resolver interpretação

- **Origem/hipótese:** v1/v2, confirmação adicional v3; adaptação semântica melhoraria ordenação.
- **Evidência:** P2-I NDCG 0,720/0,658; P3 FRX v2 0,794/original INT8 0,725; interpretação reprovada.
- **Conclusão permitida:** há sinal favorável ao recuperador nesses conjuntos, distinto de eficácia de correções.
- **Limites:** templates, relevância provisória, poucos clusters e consultas sem resposta fora das médias.
- **Consequência:** preservar o v2 e avaliar separadamente busca, abstenção e interpretação.
- **Referência:** [v2](#lab-v2), [v3](#lab-v3).

<a id="d-003"></a>
### D-003 — Menor perda, mais sintéticos e modelos maiores não bastaram

- **Origem/hipótese:** antecedentes/v2; capacidade ou dados adicionais resolveriam a generalização.
- **Evidência:** Qwen 4B ainda errou; E07 piorou no dev apesar da perda menor; 640 sintéticos pioraram o dev.
- **Conclusão permitida:** os aumentos específicos testados não corrigiram as relações.
- **Limites:** ensaios não isolam todo tamanho/arquitetura nem todos os regimes de dados.
- **Consequência:** exigir ablações e qualidade de representação antes de ampliar recursos.
- **Referência:** [avaliação inicial](FRX-AI-AVALIACAO.md), [relatório v2](FRX-AI-SCIENTIFIC-2026-10-29.md).

<a id="d-004"></a>
### D-004 — Precisão sem cobertura não resolve a tarefa

- **Origem/hipótese:** v1/v2/v4; rejeição e calibração tornariam as previsões úteis e confiáveis.
- **Evidência:** v2 pós-calibração aceitou zero; v4 reduziu falsas execuções de 27 a zero recuperando só 5/33.
- **Conclusão permitida:** os filtros melhoraram conservadorismo, sem atingir utilidade demonstrada.
- **Limites:** estimativas pequenas e protocolos distintos; não inferir uma curva única entre versões.
- **Consequência:** reportar precisão, recall/cobertura, abstenções e falsos sucessos simultaneamente.
- **Referência:** [v2](#lab-v2), [resultado v4](../research/frx_ai/v4/snapshot/final-results.json).

<a id="d-005"></a>
### D-005 — Evidências dependentes podem produzir confiança indevida

- **Origem/hipótese:** v3; ganho de informação com observações independentes guiaria diagnóstico.
- **Evidência:** sensores duplicados aumentaram erros confiantes na simulação; ruído novo degradou a política.
- **Conclusão permitida:** dependência e modelo de observação importam naquele simulador.
- **Limites:** números sintéticos não estimam causas ou probabilidades de jogos reais.
- **Consequência:** agrupar origem/discussão, preservar contradições e testar mudança de distribuição.
- **Referência:** [simulação e relatório v3](FRX-AI-MULTIDISCIPLINARY-2026-10-30.md).

<a id="d-006"></a>
### D-006 — Anotação e unidade do evento fazem parte do problema

- **Origem/hipótese:** antecedentes/v4/v6; spans e rótulos existentes seriam referência suficiente.
- **Evidência:** contexto completo corrigiu rótulos iniciais; auditorias sinalizaram ambiguidade; v6 encontrou diferenças `tried`/`running`.
- **Conclusão permitida:** parte do erro medido depende de anotação e canonicalização, além do modelo.
- **Limites:** discordância com previsão não prova rótulo errado; não houve adjudicação humana independente.
- **Consequência:** fila de revisão, ontologia explícita e rótulos versionados, sem correções silenciosas.
- **Referência:** [anotação v6](../research/frx_ai/v6/ANNOTATION.md), [relatório v4](../research/frx_ai/v4/REPORT.md).

<a id="d-007"></a>
### D-007 — Jogo da discussão e ator publicador não identificam cada evento

- **Origem/hipótese:** v5/v6; atribuição do tópico poderia preencher contexto dos acontecimentos.
- **Evidência:** duas relações corretas do v5 eram de outros jogos; citações e terceiros exigem ator separado.
- **Conclusão permitida:** metadados da página não bastam para promover eficácia ao jogo atual.
- **Limites:** v6 teve poucos pares reais de jogos explícitos; zero trocas conhecidas não valida coreferência geral.
- **Consequência:** manter contexto desconhecido e avaliar evento–jogo/ator independentemente.
- **Referência:** [auditoria v5](../research/frx_ai/v5/snapshot/posthoc-review.json), [v6](#lab-v6).

<a id="d-008"></a>
### D-008 — Execução, estado anterior, sugestão e resultado são afirmações distintas

- **Origem/hipótese:** v3–v6; separar cabeças ou verificar trechos evitaria confusão de modalidade.
- **Evidência:** 33 falsas execuções em P3; erros preexistentes/condicionais em P5; negação ainda virou sucesso em P6.
- **Conclusão permitida:** extração atômica é útil para auditar, mas não resolveu interpretação.
- **Limites:** citações literais e JSON válido garantem forma/integridade, não verdade semântica.
- **Consequência:** testar negação, sujeito, temporalidade e vínculo, com cobertura útil.
- **Referência:** [v3](#lab-v3), [v5](#lab-v5), [v6](#lab-v6).

<a id="d-009"></a>
### D-009 — Procedimentos conjuntos não concedem crédito individual

- **Origem/hipótese:** v2/v5/v6; grupos e objetivos poderiam preservar resultados mistos.
- **Evidência:** E18 melhorou representação sem confiabilidade; P5 acertou dois de três grupos propostos; P6 não recuperou nenhum dos sete anotados.
- **Conclusão permitida:** houve exemplos conhecidos/parciais, sem generalização confiável.
- **Limites:** quantidade de grupos e critérios variam; associação temporal não demonstra causalidade.
- **Consequência:** medir grupo completo e objetivos separadamente; não atribuir sucesso a cada membro.
- **Referência:** [relatório v2](FRX-AI-SCIENTIFIC-2026-10-29.md), [v5](#lab-v5), [v6](#lab-v6).

<a id="d-010"></a>
### D-010 — Candidatos completos limitam o classificador posterior

- **Origem/hipótese:** indícios v5, quantificação v6; outro verificador corrigiria as relações.
- **Evidência:** só 11/70 relações verdadeiras foram propostas completas; 61 não foram recuperadas.
- **Conclusão permitida:** no gerador e denominador P6 fixos, recall máximo do verificador é 15,7%.
- **Limites:** teto desse pipeline/dataset, não limite da IA compacta nem de outro gerador.
- **Consequência:** medir perdas por etapa e melhorar spans, resultados e grupos antes de mais classificação.
- **Referência:** [métricas v6](../research/frx_ai/v6/snapshot/final-results.json), [candidatos](../research/frx_ai/v6/pairs.py).

<a id="d-011"></a>
### D-011 — Seed inicial sem controlar infraestrutura não garante comparação

- **Origem/hipótese:** v5/v6; seeds iguais e configuração declarada bastariam para reprodução.
- **Evidência:** parser frio alterou RNG e inputs A5 não eram certificáveis; v6 restabeleceu seeds após preparação e registrou tensores.
- **Conclusão permitida:** diferenças A1–A5 não isolam causalmente arquitetura/dados.
- **Limites:** refits exatos locais não garantem igualdade binária em outra stack.
- **Consequência:** registrar hashes/versões, seeds após inicializações e separar falha de infraestrutura de erro de modelo.
- **Referência:** [RNG v5](../research/frx_ai/v5/snapshot/seed-audit.json), [reprodução v6](../research/frx_ai/v6/snapshot/reproduction.json).

<a id="d-012"></a>
### D-012 — Inferência pequena é viável, mas não certifica inteligência diagnóstica

- **Origem/hipótese:** v2–v6; artefatos locais poderiam viabilizar integração futura sem serviço de inferência.
- **Evidência:** ONNX no v2/v4; v6 ≈38,7 MB ativos e execução sem Torch; memória contextual passou contratos.
- **Conclusão permitida:** existe protótipo local e infraestrutura útil para investigação assistida.
- **Limites:** Python/dependências fora do tamanho; casos pequenos; não há integração Rust ou diagnóstico autônomo validado.
- **Consequência:** preservar componentes úteis e só integrar capacidades após avaliação específica.
- **Referência:** [README v6](../research/frx_ai/v6/README.md), [benchmark](../research/frx_ai/v6/snapshot/efficiency.json).

<a id="d-013"></a>
### D-013 — O verificador não era o gargalo

- **Origem/hipótese:** v7; D-010 apontava candidatos, faltava medir o teto do verificador.
- **Evidência:** o mesmo L1 passou de 9/70 a 41/70 (com filtro) e 57/70 (sem filtro) com candidatos-oráculo; em P7-M, 16/30 e 24/30.
- **Conclusão permitida:** nesses conjuntos, a maior parte da perda é anterior à decisão.
- **Limites:** o oráculo tem poucos distratores e rótulos provisórios; é teto diagnóstico, não desempenho alcançável.
- **Consequência:** priorizar detecção de resultados e eventos antes de novos classificadores.
- **Referência:** [v7](#lab-v7), [relatório](../research/frx_ai/v7/REPORT.md).

<a id="d-014"></a>
### D-014 — Separar proposta de decisão amplia candidatos, com ganho menor em dados novos

- **Origem/hipótese:** v7; modalidade como atributo e resultados por oração cobririam mais.
- **Evidência:** 69% em P6 conhecido, contra 43% em P7-M e 42% em J7, ainda acima de 13% e 15% de v6.
- **Conclusão permitida:** há ganho real nesses testes e otimismo real no desenvolvimento.
- **Limites:** intervalo do ganho inclui zero em P7-M; amostras enriquecidas; mesmo autor nos rótulos e regras.
- **Consequência:** medir sempre em coorte nova; não reportar cobertura de desenvolvimento como capacidade.
- **Referência:** [resultados](../research/frx_ai/v7/snapshot/final-results.json).

<a id="d-015"></a>
### D-015 — Propor o grupo não recupera o grupo

- **Origem/hipótese:** v7; representar a estrutura recuperaria conjuntos sem crédito individual.
- **Evidência:** em J7, 6/20 propostos e 2/20 recuperados, contra 3/3 de v6; crédito individual indevido persistiu.
- **Conclusão permitida:** a hipótese foi refutada nesse desafio; a incerteza é representável, decidir o grupo não está resolvido.
- **Limites:** J7 é enriquecido e selecionado por regra lexical; poucos casos por tipo de estrutura.
- **Consequência:** tratar pertencimento ao grupo como problema próprio, com negação explícita de atribuição pelo autor.
- **Referência:** [v7](#lab-v7), [D-009](#d-009).

<a id="d-016"></a>
### D-016 — Pedidos de mantenedor ensinam triagem, não diagnóstico

- **Origem/hipótese:** v7; uma política aprendida de demonstrações superaria baselines.
- **Evidência:** top-1 de 58,3% para a maioria e 54,8% para o selecionado; o mantenedor pede log em 51,5% sem log e 56,7% com log.
- **Conclusão permitida:** nesse conjunto, o pedido quase não depende do estado extraído; não houve ganho aprendido.
- **Limites:** o estado é limitado por regex e não lê logs; imitar não mede utilidade.
- **Consequência:** rotular utilidade da verificação, não o pedido.
- **Referência:** [D7](../research/frx_ai/v7/snapshot/selector-final.json).

<a id="d-017"></a>
### D-017 — Texto de template contamina rótulo, estado e sintoma

- **Origem/hipótese:** v7; texto de issue seria relato do usuário.
- **Evidência:** comentários de transferência geravam falsos pedidos; dicas de `PROTON_LOG` marcavam log presente em 344/417; `Video driver version:` gerou o sintoma de vídeo em C7.
- **Conclusão permitida:** o modelo de issue é uma fonte recorrente de erro; duas ocorrências foram detectadas por auditoria no treino e uma só depois do teste.
- **Limites:** outras contaminações podem existir; a lista não é exaustiva.
- **Consequência:** auditar amostras reais de rótulos e separar template de relato antes de medir.
- **Referência:** [relatório](../research/frx_ai/v7/REPORT.md).

<a id="d-018"></a>
### D-018 — A proveniência dos rótulos é uma restrição de pesquisa

- **Origem/hipótese:** v7; ensino por modelos mais capazes exigiria verificar autorização.
- **Evidência:** os Termos Comerciais da Anthropic (D.4) tratam de treinar modelos concorrentes; rótulos de v2–v6 são do agente, e parte treinou modelos.
- **Conclusão permitida:** há uma questão contratual aberta, a ser decidida pelo responsável; não é análise jurídica.
- **Limites:** este laboratório não determina o enquadramento da FRX AI nem o alcance do termo.
- **Consequência:** no v7, saída do agente só avalia; proveniência acompanha todo dado de treino.
- **Referência:** [literatura v7](../research/frx_ai/v7/LITERATURE.md).
- **Errata:** a evidência desta descoberta foi corrigida em [E-001](#errata-e-001).

<a id="d-019"></a>
### D-019 — Os rótulos de v1–v6 vieram do Codex, e todos os pesos dependem deles

- **Origem/hipótese:** auditoria pedida pelo responsável em 30/10/2026.
- **Evidência:** 13 scripts de anotação e a lista inline do v3 aparecem na sessão do Codex (`gpt-6.1-sol`) e em nenhuma do Claude antes da leitura do v7; 15/15 `train_hash` do v2 conferem. 50 de 51 pesos auditados dependem desses rótulos ou programas.
- **Conclusão permitida:** a proveniência é conhecida por modelo e dataset; a autorização contratual segue em aberto.
- **Limites:** revisão fora das sessões não é detectável; dependências fora do v2 vêm de leitura do código e manifestos.
- **Consequência:** congelamento e nenhum treino novo com esses rótulos até concluir a auditoria.
- **Referência:** [auditoria](../research/frx_ai/audit/PROVENANCE-AUDIT.md), [matriz](../research/frx_ai/audit/provenance-matrix.json).

<a id="d-020"></a>
### D-020 — Não há rótulo com revisão humana independente em v1–v7

- **Origem/hipótese:** auditoria; generaliza a ressalva de Q-005.
- **Evidência:** todas as declarações de proveniência dizem que não houve revisão; as filas de revisão (15, 57 e 52 itens) não têm resposta; "manual" e "Human" em registros antigos designam um modelo.
- **Conclusão permitida:** todas as métricas de v1–v7 medem concordância com rótulos provisórios de agente.
- **Limites:** não prova que as métricas estejam erradas, só que não têm validação independente.
- **Consequência:** a próxima prioridade é um benchmark com dois anotadores humanos, não outro classificador.
- **Referência:** [plano de avaliação independente](../research/frx_ai/audit/INDEPENDENT-EVALUATION-PLAN.md).

<a id="d-021"></a>
### D-021 — Gabaritos antigos não medem um detector isolado de resultados

- **Origem/hipótese:** v8; medir o detector do v7 sozinho.
- **Evidência:** os gabaritos marcam só resultados ligados a ações, então sintomas reais aparecem como falsos positivos. Em O8, com a tarefa redefinida, o v8 teve 3 falsos positivos contra 6 e 22/32 contra 28/32 de cobertura.
- **Conclusão permitida:** separar estado de resultado reduz afirmações falsas nestes dados, com menor cobertura.
- **Limites:** rótulos H0; a cobertura ficou abaixo do critério; o E07 sem filtro não fica claramente atrás; houve sobreajuste ao desenvolvimento.
- **Consequência:** nenhuma regra nova antes do gabarito humano.
- **Referência:** [relatório v8](../research/frx_ai/v8/REPORT.md).

<a id="d-022"></a>
### D-022 — Procedimentos conjuntos são um problema de extração, não de representação

- **Origem/hipótese:** H-J1 do v8, refutada.
- **Evidência:** os esquemas do v7 e do v8 expressam os 20 grupos do J7, e mesmo assim o v8 acertou 1/14 em dados novos e o v7, 0/14.
- **Conclusão permitida:** o gargalo é encontrar membros e resultado.
- **Limites:** apenas 14 grupos; rótulos H0.
- **Consequência:** a representação fica; não se atribui crédito individual em grupos. Melhorias exigem dados com revisão humana.
- **Referência:** [relatório v8](../research/frx_ai/v8/REPORT.md).

<a id="d-023"></a>
### D-023 — A vantagem na próxima verificação vem sobretudo de notar logs já fornecidos

- **Origem/hipótese:** H-T1 do v8.
- **Evidência:** top-3 de 25/26 contra 8/26 e 10/26; `read-attached-log`, criada no v8, foi a escolha em 16/26. Ao fundir pedir e ler log, o top-1 fica em 21/26 contra 18/26, com IC [0,0; 29,2] pontos.
- **Conclusão permitida:** respeitar o que já foi fornecido evita pedidos redundantes. Isso é regra, não diagnóstico inteligente.
- **Limites:** mesmo autor na política e no gabarito; análise de sensibilidade posterior; o protótipo não lê anexos.
- **Consequência:** nenhum modelo demonstrou vantagem na seleção de verificações.
- **Referência:** [análise posterior](../research/frx_ai/v8/snapshot/posthoc.json).

<a id="d-024"></a>
### D-024 — Acompanhar a conversa é o elo mais fraco para investigar casos reais

- **Origem/hipótese:** frente T do v8.
- **Evidência:** 7/18 resultados anotados encontrados, com 31 extraídos; a decisão mudou corretamente em 6/12 casos; há só 6 hipóteses explícitas em 12 threads.
- **Conclusão permitida:** threads reais raramente seguem o esquema completo, e as regras atuais não reconstroem bem o estado do caso.
- **Limites:** 12 threads; rótulos H0.
- **Consequência:** medir resultado encontrado por turno como objetivo próprio.
- **Referência:** [relatório v8](../research/frx_ai/v8/REPORT.md).

<a id="d-025"></a>
### D-025 — Seguir pistas por regra não superou uma pesquisa única

- **Origem/hipótese:** H9 do v9.
- **Evidência:** conceito no relatório: C 1/17, B 1/17, B2 1/17, F 0/17, A 0/17; C − B = 0,0 [0,0; 0,0]. Uma de 30 consultas posteriores trouxe conceito novo.
- **Conclusão permitida:** com estas regras e este corpus, a iteração não mostrou vantagem.
- **Limites:** gabarito H0, n pequeno, um só repositório. Não refuta a iteração com um leitor melhor.
- **Consequência:** H9 não sustentada. A comparação deve ser repetida depois de melhorar o leitor.
- **Referência:** [relatório v9](../research/frx_ai/v9/REPORT.md).

<a id="d-026"></a>
### D-026 — O gargalo é entender o que foi lido

- **Origem/hipótese:** replays de diagnóstico, depois da leitura única.
- **Evidência:** com a resposta como consulta, conceito lido em 16/17 e no relatório em 1/17; termos raros do corpus também não levaram nada ao relatório.
- **Conclusão permitida:** melhorar a busca sozinha não resolveria este problema nestes dados.
- **Limites:** análise posterior em dados conhecidos; conceitos por texto literal.
- **Consequência:** a próxima prioridade é o leitor de evidências, medido sozinho.
- **Referência:** [replays](../research/frx_ai/v9/snapshot/replays.json).

<a id="d-027"></a>
### D-027 — Pistas rastreáveis ainda podem enganar

- **Origem/hipótese:** revisão H0 dos relatórios do v9.
- **Evidência:** 0 corretos, 4 parciais e 5 enganosos em 9; citações literais em 57/57.
- **Conclusão permitida:** proveniência correta não implica pertinência. Nenhum relatório afirmou causa confirmada.
- **Limites:** revisão do mesmo autor.
- **Consequência:** a aplicabilidade deve ser medida à parte da proveniência.
- **Referência:** [relatório v9](../research/frx_ai/v9/REPORT.md).

<a id="d-028"></a>
### D-028 — Atualizar o caso entre turnos é viável por regra

- **Origem/hipótese:** teste de continuidade do v9.
- **Evidência:** abandona o pedido de log em 4/4; não repete verificação refutada em 4/4; pesquisa o erro concreto em 3/4; nunca confirma causa em 4/4.
- **Conclusão permitida:** o estado do caso incorpora informação nova em sequências preparadas.
- **Limites:** fixtures e catálogo do mesmo autor. Não mede diagnóstico.
- **Consequência:** é base útil para o próximo ciclo. Em threads reais, D-024 continua válida.
- **Referência:** [relatório v9](../research/frx_ai/v9/REPORT.md).

<a id="questoes"></a>
## VIII. Registro de hipóteses e questões abertas

Tudo nesta tabela é **proposta futura**, não resultado implementado. Os critérios
devem ser fixados antes de ver uma nova aceitação; valores finais exigem um
protocolo e revisão do orçamento.

| ID | Problema e evidência | Hipótese/experimento sugerido | Critério proposto |
|---|---|---|---|
| Q-001 | Candidatos completos 11/70; D-010 | Aprender spans/argumentos canônicos, medir separadamente evento, resultado e grupo; comparar com gerador atual | Ganho de recall de candidatos em novas discussões sem explosão de candidatos falsos; efeito ponta a ponta medido |
| Q-002 | Nenhum dos sete grupos P6 recuperado; D-009 | Anotar coreferência e objetivos de procedimentos compostos; controles com argumentos de referência | Aumentar relações de grupo completo sem crédito individual indevido |
| Q-003 | Falsos sucessos e confusão de modalidade; D-008 | Contrastes mínimos e verificação ação–resultado com sujeito/negação/tempo explícitos | Reduzir erros críticos mantendo recall útil; intervalo e curva risco–cobertura publicados |
| Q-004 | Contexto do tópico insuficiente; D-007 | Coletar casos reais de múltiplos jogos, autores/citações e versões; avaliar identidade antes do resultado | Precisão e recall de identidade em amostra suficiente, sem preencher desconhecidos por conveniência |
| Q-005 | Rótulos provisórios e problemas de trigger; D-006 | Revisão humana independente de spans/ontologia, adjudicação e medição de concordância | Dataset versionado por jogo/thread, critérios explícitos e teste novo sem derivações do dev |
| Q-006 | Diagnóstico só por controles selecionados | Casos de incidente com logs, etapas de informação e marcos separados da entrada; comparar perguntas simples e política aprendida | Próximas verificações pertinentes, fechamento de evidência e causas desconhecidas em incidentes independentes |
| Q-007 | Busca pode ordenar sem resposta relevante | Avaliar consultas sem relevante, português livre e drift de versão com FRX v2 como baseline | Recuperação temática e detecção de ausência de resposta medidas separadamente |
| Q-008 | Aprendizado local ainda é memória, não causalidade | Casos/contextos controlados, dependências e resultados observados com qualidade declarada; avaliar antes de otimizar intervenções | Sem transferência indevida nem posterior inventado; calibração e retenção sob mudança de ambiente |
| Q-009 | Resultado não detectado afeta 41/70; D-013 | Detector de afirmação de estado do jogo por oração, avaliado sozinho em coorte nova | Recall de resultados com falsos sucessos medidos, sem tratar objetivo ou pergunta como resultado |
| Q-010 | Demonstrações medem prática; D-016 | Anotar trajetórias pedido → resposta → conclusão e rotular quais verificações separaram hipóteses, lendo logs reais | Política supera sempre-log em utilidade, em discussões novas, com revisão humana |
| Q-011 | Proveniência de rótulos; D-018, D-019 | Decisão de 30/10/2026 na [seção XIII](#proveniencia): preservar e congelar. Falta esclarecer os termos e fazer anotação humana independente | Todo dado de treino com proveniência autorizada e declarada |
| Q-012 | Como os termos da OpenAI e da Anthropic se aplicam aos dados concretos; D-019 | Ler as páginas da OpenAI, que recusaram leitura automática; pedir esclarecimento por escrito | Posição documentada de cada provedor ou decisão registrada do responsável |
| Q-013 | Sem benchmark independente; D-020 | Fases 0–1 do [plano](../research/frx_ai/audit/INDEPENDENT-EVALUATION-PLAN.md) | Benchmark selado H2, com α ≥ 0,80 nos campos decisivos |
| Q-014 | Compreensão de logs reais e trajetórias completas; D-016, D-023, D-024 | Fases 3–4 do plano: problema, evidência, hipótese, verificação, resultado e decisão seguinte | Vantagem pareada acima do melhor comparador, replicada, sem causa inventada |
| Q-015 | Detector de resultados em gabarito humano; D-021 | Anotar o piloto; depois comparar v8, v7, `reports.rs` e pesos congelados | Precisão e cobertura com limites pré-registrados em avaliação H2 |
| Q-016 | Anexos públicos; D-023 | Decisão do responsável sobre ler logs anexados em issues públicas, com redação | Política de uso e privacidade registrada |
| Q-017 | Leitor de evidências; D-026 | Medir sozinho, com o documento certo já dado, regras contra modelo local congelado | Afirmações pertinentes no relatório, em gabarito com revisão humana |
| Q-018 | Pesquisa iterativa; D-025 | Repetir depois do leitor, com orçamento igual e mais de um repositório | C acima de B e F, com IC pareado acima de 0 |
| Q-019 | Modelo gerativo local não testado | Avaliar modelo aberto congelado, com licença adequada, como leitor antes de planejador | Contribuição medida contra regras |

Mais uma grade de classificadores sobre o mesmo teste conhecido não responde a
Q-001/Q-005. Bayesian Optimization, bandits, SAT/SMT e otimização multiobjetivo
continuam possibilidades investigativas; os registros não demonstram sua adoção
como mecanismo de correção do FRX AI.

<a id="referencias"></a>
## IX. Referências científicas consolidadas

Seleção das fontes efetivamente investigadas nos laboratórios. Títulos, anos e
autorias abreviadas abaixo foram conferidos nas páginas primárias em 30/10/2026;
as métricas do FRX vêm dos registros locais. “Inspiração” não significa
reprodução do artigo. As bibliografias [v2](../research/frx_ai/v2/LITERATURE.md),
[v3](../research/frx_ai/v3/LITERATURE.md), [v4](../research/frx_ai/v4/LITERATURE.md),
[v5](../research/frx_ai/v5/LITERATURE.md), [v6](../research/frx_ai/v6/LITERATURE.md) e
[v7](../research/frx_ai/v7/LITERATURE.md) preservam as demais leituras e os limites de disponibilidade de código.

| ID | Referência primária | Área e relação com o FRX AI |
|---|---|---|
| R-01 | Yao et al. (2019), [DocRED: A Large-Scale Document-Level Relation Extraction Dataset](https://aclanthology.org/P19-1074/) | Relações documentais; inspirou preservar contexto e evidências, sem usar Wikipedia como rótulo de gaming |
| R-02 | Zhong e Chen (2021), [A Frustratingly Easy Approach for Entity and Relation Extraction](https://aclanthology.org/2021.naacl-main.5/) — PURE | Separação de entidades/relações; nossos geradores determinísticos não reproduzem seus dois encoders |
| R-03 | Qin et al. (2021), [ERICA: Improving Entity and Relation Understanding for Pre-trained Language Models via Contrastive Learning](https://aclanthology.org/2021.acl-long.260/) | Contraste relacional; houve perdas contrastivas próprias, sem reprodução do pré-treino ERICA |
| R-04 | Tunstall et al. (2022), [Efficient Few-Shot Learning Without Prompts](https://arxiv.org/abs/2209.11055) — SetFit | Adaptação compacta; leitura motivadora, não implementação integral do método |
| R-05 | Ratner et al. (online 2019; volume 2020), [Snorkel: rapid training data creation with weak supervision](https://doi.org/10.1007/s00778-019-00552-1) | Supervisão fraca; v2 fez ablação própria sem o denoising generativo completo de Snorkel |
| R-06 | Guo et al. (2017), [On Calibration of Modern Neural Networks](https://proceedings.mlr.press/v70/guo17a.html) | Temperature scaling efetivamente experimentado no v2; calibração não certificou associação correta |
| R-07 | Northcutt et al. (2021), [Confident Learning: Estimating Uncertainty in Dataset Labels](https://research.google/pubs/confident-learning-estimating-uncertainty-in-dataset-labels/) | Ruído de rótulos; motivou auditoria, sem tratar previsão como verdade ou executar poda automática equivalente |
| R-08 | Li, Wang e Du (2025), [Efficient Document-level Event Relation Extraction](https://aclanthology.org/2025.repl4nlp-1.7/) | Pares candidatos seguidos de verificação; princípio adaptado e medido, não reprodução completa |
| R-09 | Zhu et al. (2026), [SuperDRI: Graph-Guided Autonomous Troubleshooting for Cloud Databases](https://www.microsoft.com/en-us/research/publication/superdri-graph-guided-autonomous-troubleshooting-for-cloud-databases/) | Investigação por grafo; inspiração para contexto/etapas, sem agentes de nuvem ou queries executáveis |
| R-10 | Fang et al. (2026), [DrDEE: A Deconstruction-Reconstruction Framework for Document-Level Event Extraction](https://aclanthology.org/2026.findings-emnlp.1723/) | Verificação atômica/reconstrução; adaptação conceitual v5/v6, sem reproduzir o modelo financeiro |
| R-11 | Hu et al. (2025), [Joint Learning Event-Specific Probe and Argument Library with Differential Optimization for Document-Level Multi-Event Extraction](https://aclanthology.org/2025.findings-naacl.42/) — EPAL | Separação de eventos/argumentos investigada no v6; não implementamos EPAL |
| R-12 | Chen, Li e Zhu (2025), [Employing Discourse Coherence Enhancement to Improve Cross-Document Event and Entity Coreference Resolution](https://aclanthology.org/2025.acl-long.1134/) | Coreferência investigada; regras locais não equivalem ao método aprendido |
| R-13 | Zerong et al. (2025), [A Systematic Survey of Claim Verification: Corpora, Systems, and Case Studies](https://aclanthology.org/2025.findings-emnlp.1170/) | Fundamentação da avaliação de afirmações; não fornece validação humana para os rótulos FRX |
| R-14 | Wang et al. (2026), [Cloud-OpsBench: A Reproducible Benchmark for Agentic Root Cause Analysis in Cloud Systems](https://arxiv.org/abs/2603.00468) | Snapshots, trajetórias e evidência; replay adaptado, sem executar o benchmark de nuvem |
| R-15 | Rao e Daumé III (2018), [Learning to Ask Good Questions: Ranking Clarification Questions using Neural Expected Value of Perfect Information](https://aclanthology.org/P18-1255/) | Candidatos de posts parecidos; recuperação de casos implementada no v7, sem o modelo EVPI |
| R-16 | Dalvi Mishra et al. (2019), [Everything Happens for a Reason: Discovering the Purpose of Actions in Procedural Text](https://aclanthology.org/D19-1457/) | Dependências entre passos; motivou representar procedimentos, sem reproduzir XPAD |
| R-17 | Su et al. (2026), [Suggest-Verify-Revise: A Three-Stage Document-Level Event Causality Identification with Narrative Consistency](https://aclanthology.org/2026.acl-long.871.pdf) | Estágios sugerir e verificar; leitura de princípio, sem implementação |
| R-18 | Hagen et al. (2025), [Investigating Counterclaims in Causality Extraction from Text](https://arxiv.org/abs/2510.08224) | Contra-afirmações causais; motivou atenção a negativas, sem treino equivalente |
| R-19 | Yu, Li e Wang (2019), [Detecting Causal Language Use in Science Findings](https://aclanthology.org/D19-1473/) | Linguagem causal × correlacional; motivou não afirmar causalidade, sem classificador treinado |
| R-20 | Fontes oficiais de termos, listadas na [seção XIII](#proveniencia) | Pendência de proveniência; não é base científica nem conclusão jurídica |
| R-21 | Yao et al. (2023), [ReAct](https://arxiv.org/abs/2210.03629) | Intercalar ações e observações; só a estrutura foi usada, sem LLM |
| R-22 | Trivedi et al. (2023), [IRCoT](https://aclanthology.org/2023.acl-long.557/) | Recuperação intercalada; motivou medir o ganho de cada busca; o ganho não foi reproduzido |
| R-23 | OWASP, [LLM Prompt Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html) | Permissões e limites fora do modelo |

A disponibilidade de código descrita nos relatórios é histórica: falha de acesso
ou ausência de link na consulta não prova inexistência permanente. Licenças de
modelos, dados de treino e comentários são questões distintas; acesso público e
checksum não autorizam redistribuição indiscriminada nem atestam segurança de um patch.

<a id="reproducibilidade"></a>
## X. Reprodutibilidade, disponibilidade e lacunas

### Como localizar os procedimentos

Este diário não reedita comandos de treino como se tivessem sido reexecutados.
As entradas v1–v6 abaixo foram conferidas por leitura dos scripts/READMEs, sem novos
treinamentos naquela consolidação. A entrada v7 registra execuções do próprio laboratório.

| Etapa | Guia operacional | Inferência | Avaliação/treino e dependências externas |
|---|---|---|---|
| v1 | [README raiz](../research/frx_ai/README.md) | `prototype.py`: Python, ONNX Runtime, tokenizers e pesos MiniLM/NLI | Corpus anotado 100/50/80 e cabeças no cache externo; coletor não recria sozinho os rótulos |
| v2 | [README v2](../research/frx_ai/v2/README.md) | `prototype.py`, modelos ONNX por objetivo/recuperação | `data.py`, `replay.py` e scripts de treino; modelos, tokenizers, corpus por hash e checkpoints externos; PyTorch no treino |
| v3 | [README v3](../research/frx_ai/v3/README.md) | `case_review.py`, v2 preservado, memória opcional | `reproduce.py` e `hydrate_final.py`; exige artefatos/dados v2 e textos originais para conferir offsets |
| v4 | [README v4](../research/frx_ai/v4/README.md) | `run.py`, detector ONNX e recuperador v2 | `hydrate.py`, `reproduce.py`, v2/v3 e cache de corpos; requisitos de treino separados |
| v5 | [README v5](../research/frx_ai/v5/README.md) | `run.py`, Stanza e FRX v2; spaCy como controle | `hydrate.py`, `train_atomic.py`, `train_linker.py`; v4, modelos/manifests sintáticos, dados e checkpoints externos |
| v6 | [README v6](../research/frx_ai/v6/README.md) | `investigate.py`, spaCy, ONNX v2 e JSON do verificador; sem Torch no caminho selecionado | `rehydrate.py`, `final.py`, `learning.py`, `neural.py`; caches antigos, corpos, tokenizers e pesos externos |
| v7 | [README v7](../research/frx_ai/v7/README.md) | `frx_lab.py`, `procedures.py`, `investigate.py`, parser spaCy, recuperador v2 e índice local | `collect.py`, `reserve.py`, `freeze.py`, `final.py`, `oracle.py`, `selector.py`, `c7.py`; corpus e rótulos no cache externo |

1. **Reprodução de inferência:** executa os artefatos preservados; um clone sem
   os modelos não basta. Preparação explícita é diferente de download implícito
   durante a inferência. Nenhum modelo foi incorporado ao instalador por essas rodadas.
2. **Reprodução de avaliação:** precisa de texto idêntico, offsets, rótulos,
   seleção e hashes. Fonte pública editada/removida pode exigir o cache original.
   V5 registrou paridade de cinco comparadores; v6, de seis em diretório separado.
3. **Reprodução de treinamento:** requer pesos iniciais, dados, seeds, versões,
   ordem de preparação e seleção. V5 documenta confusão de RNG; v6 registra
   tensores e dois refits lineares com diferença máxima de coeficientes zero.
   Isso não garante pesos idênticos em toda plataforma.
4. **Disponibilidade dos dados:** snapshots fornecem metadados/anotações, sem
   corpus integral ou checkpoints. Consultas explícitas via API têm limites e
   verificam hashes. Um `test_*.py` aprovado não substitui avaliação linguística.

### Lacunas e diferenças que não devem ser apagadas

| Registro | Situação documentada e tratamento neste diário |
|---|---|
| Nome v1 | Não existe diretório `v1/`; o README raiz denomina seus resultados como v1. Qwen/Rust anteriores são antecedentes, não o mesmo dataset P1 |
| Precisão de 9/9 do Rust | P0-B pequeno, com intervalo amplo; não substitui os 3/4 de P1 ou avaliações posteriores com outros procedimentos |
| Normalização NLI v1 | Comparador de dev invalidado; detector selecionado com atributos separados preservado. Não reaproveitar a comparação inválida |
| V2 FP32/INT8 | `retrieval-final-v2.json` contém resultados distintos: `results[0]` semântico ≈0,695 NDCG e `native_runtime` ≈0,720. A tabela P2-I identifica a variante nativa, não escolhe números intercambiáveis |
| V3 memória/fluxo final | Montagem assistida posterior ao teste do challenger; não declarar que o benchmark congelado validou todo esse fluxo |
| V3 RAM | Relatório preserva ≈830 MiB no lançamento via `ru_maxrss` e ≈216 MiB no processo replicado via `/proc`; instrumentos/processos diferentes, sem apagar um registro |
| V4/v5/v6 comparados | Zero relações no P4 original, 1/1 do v4 em P5 e 4/4 do v5 em P6-M são reavaliações em protocolos/dados distintos, não inconsistência resolvida por trocar a métrica histórica |
| V5 associações | Seis correspondências linguísticas incluem duas de outros jogos; preservar o total e a auditoria contextual, sem chamar todas de eficácia no jogo atual |
| V5 treinos A1–A5 | RNG e composição A5 não certificável; não inferir efeitos causais de ablações apenas desses números |
| V6 congelamento | Exceção de offsets seguida de reparo mínimo documentado antes de métricas; manter selo original, emenda e rótulos travados |
| V6 falsas execuções | Critério estrito inclui cabeça verbal, evento extra e possível omissão de rótulo. Publicar resultados originais e revisão separada, sem corrigir a favor do modelo |
| V7 C7 e correções | `c7-results.json` é a leitura original; `c7-known-replay.json` vem depois de corrigir `Video driver` e logs anexados. Regras 8/24 e híbrido 4/24 viram 13/24 e 16/24 no replay, que não é avaliação independente |
| V7 proveniência | Rótulos de v2–v6 são do agente e parte treinou modelos; v7 não treina com eles. Enquadramento contratual não decidido neste diário. Agente identificado na [errata E-001](#errata-e-001) |
| V8 rótulos | Rótulos H0 do mesmo agente que escreveu as regras; não são validação humana. Cobertura abaixo do critério; T8 depende fortemente de uma regra nova |
| V9 avaliação | Gabarito e julgamentos H0 do autor das regras, relevância só por título. Fronteira entre leitura congelada e replays preservada. Ao vivo não é reproduzível, e 13 execuções inválidas foram descartadas e registradas |
| Alcance da consolidação Git | Na inspeção de 30/10/2026, `v5/` e `v6/` estão presentes, mas não rastreados. Seus arquivos não entram no commit documental solicitado. Links existem neste working tree; um clone só desse commit pode não conter tais artefatos |

A última limitação de versionamento não autoriza incluir laboratórios, modelos
ou dados em um commit de documentação. Quando forem versionados separadamente,
registrar sua disponibilidade no changelog, preservando a síntese já publicada.

**Atualização de disponibilidade — 30/10/2026.** Os laboratórios v5 e v6 foram
versionados separadamente após a consolidação inicial: código/metadados em
`d0f5876` e `6448aac`, documentação em `f5e04ae` e `a8be090`, respectivamente.
A situação não rastreada descrita acima é o registro da inspeção inicial; os
artefatos referenciados agora integram o histórico desta branch. Corpos de
comentários, modelos, checkpoints e caches continuam externos e não foram
incluídos nesses commits. Nenhuma métrica ou conclusão científica foi alterada.

<a id="manutencao"></a>
## XI. Como atualizar o RXAI

Ao concluir v7 ou outra etapa:

1. Ler este diário e identificar o último estado registrado.
2. Examinar o novo laboratório, relatório, código pertinente e snapshots.
3. Acrescentar uma subseção de versão ao histórico com os mesmos onze campos:
   objetivo, hipóteses, fundamento, metodologia, implementação, experimentos,
   resultados, descobertas, falhas/limites, decisões e artefatos.
4. Acrescentar um ID de avaliação, com unidade, amostra, splits, anotador,
   critérios, seleção/congelamento e estatuto de teste novo ou replay.
5. Atualizar a linha do tempo e o estado atual; não reescrever a história como
   progresso contínuo se houver regressão ou resultado inconclusivo.
6. Acrescentar descobertas com o próximo `D-NNN` e atualizar questões `Q-NNN`,
   mantendo IDs antigos. Uma hipótese refutada recebe evidência e estado, não exclusão.
7. Acrescentar referências e distinguir implementação de leitura/inspiração.
8. Revisar todas as métricas: dataset, denominador, variante, fonte, protocolo,
   cobertura e erros críticos. Comparações exigem mesma tarefa e coorte.
9. Validar links/âncoras e disponibilidade real dos artefatos. Não incluir pesos,
   corpos privados, caches ou alterações de produção por conveniência documental.
10. Acrescentar entrada datada ao changelog, explicando adições e eventuais erratas.

**Regras permanentes:** não apagar tentativas malsucedidas; não substituir números
históricos pelos de outra avaliação; não elevar rótulos provisórios a verdade
humana; não tratar teste conhecido como independente; não renumerar descobertas.
Uma errata deve citar a afirmação anterior, motivo, artefato e resultado correto,
preservando a rastreabilidade. Hipótese, inferência e plano futuro continuam
identificados separadamente. Novas seções são acrescentadas sem mover laboratórios
nem duplicar integralmente seus relatórios.

<a id="changelog"></a>
## XII. Histórico de alterações do diário

| Data | Alteração | Base e limite |
|---|---|---|
| 30/10/2026 | Criação da edição 1 do RXAI; consolidação dos antecedentes e v1–v6; protocolos, linha do tempo, D-001–D-012, Q-001–Q-008, bibliografia e manutenção | Leitura de relatórios, READMEs, snapshots e código relevante; sem novo treino/benchmark. Disponibilidade não rastreada de v5/v6 registrada. Adicionado índice em `docs/README.md` |
| 30/10/2026 | Atualização editorial da disponibilidade dos laboratórios v5/v6 após seu versionamento separado | Registrados commits de código/metadados e documentação; preservado o estado histórico da inspeção inicial. Pesos, corpos e caches permanecem externos; resultados científicos inalterados |
| 30/10/2026 | Edição 2: entrada v7, protocolos P7-M, P7-J, J7, D7 e C7, D-013–D-018, Q-009–Q-011, R-15–R-19 | Base nos artefatos de `research/frx_ai/v7/`. Laboratório ainda não versionado em Git nesta edição: os links existem neste working tree. Resultados de v1–v6 inalterados |
| 30/10/2026 | Edição 3: seção XIII, errata E-001, D-019, D-020, Q-012–Q-014, R-20; Q-011 atualizada | Base em `research/frx_ai/audit/`, ainda não versionado em Git. Sem treino novo; nenhuma métrica histórica alterada; sem conclusão jurídica |
| 30/10/2026 | Edição 4: entrada v8, protocolos O8, T8 e L8, D-021–D-024, Q-015–Q-016; decisões do responsável e termos da OpenAI na seção XIII | Base em `research/frx_ai/v8/` e `research/frx_ai/benchmark/`, não versionados. Rótulos H0; nenhum treino; erratas preservadas. Termos sem leitura direta, com a origem de cada item indicada |
| 31/10/2026 | Edição 5: entrada v9, protocolos R9 e R9-L, D-025–D-028, Q-017–Q-019, R-21–R-23 | Base em `research/frx_ai/v9/`, não versionado. Rótulos H0; nenhum treino; conclusões anteriores preservadas |

<a id="proveniencia"></a>
## XIII. Proveniência: decisão, auditoria e pendências

### Decisão do responsável (30/10/2026)

1. Nenhum laboratório, modelo ou dataset de v1–v7 é descartado.
2. Modelos com dados de proveniência incerta ficam só em pesquisa interna, sem integração ao aplicativo e sem novos treinos com os mesmos rótulos até concluir a auditoria.
3. Os modelos anteriores não são declarados proibidos nem inválidos sem fundamento.
4. A próxima prioridade não é outro classificador: é um benchmark independente com revisão humana e um detector de resultados avaliado sozinho. Em paralelo, logs reais e trajetórias completas de diagnóstico.
5. Heurísticas e capacidades aprendidas continuam separadas. Nenhum modelo demonstrou vantagem consistente na seleção de verificações, e regras fixas não são anunciadas como diagnóstico inteligente.

### Resultado da auditoria

Identificadores de sessão, caminhos pessoais e detalhes de conta foram omitidos da versão pública.

Detalhes em [PROVENANCE-AUDIT.md](../research/frx_ai/audit/PROVENANCE-AUDIT.md).

| Origem | Rótulos | Pesos salvos que dependem deles |
|---|---|---|
| Codex (OpenAI, `gpt-6.1-sol`), conta pessoal, direto | Rótulos diretos de v1–v6 | 50 de 51 auditados, somados aos programas abaixo |
| Codex, programas | `weak-events`, sintéticos, contra-afirmações, pares de metas | Vários; ver a matriz |
| Claude (Opus 5.5 e Haiku 5.5), conta pessoal, direto | P7, J7, C7; antecedentes de 29/10 | Nenhum |
| Claude, programas | Regras de D7 | Nenhum salvo; `selector.py` treinou nelas no v7 |
| Fontes verificáveis | Texto e metadados públicos, não categorias | Não se aplica |
| Revisão humana independente | Nenhum | Nenhum |

O aplicativo não contém modelo da pesquisa. Contém regras fixas em `src-tauri/src/fixes/reports.rs`, medidas com rótulos do Claude e sem validação independente.

<a id="errata-e-001"></a>
### Errata E-001

- **Afirmação anterior:** D-018, a entrada v7, `v7/PROTOCOL.md`, `v7/REPORT.md` e `v7/LITERATURE.md` apresentaram a questão a partir dos Termos Comerciais da Anthropic (D.4), como se os rótulos de v2–v6 viessem do agente que escrevia o v7.
- **Motivo:** os rótulos de v1–v6 foram escritos pelo Codex, e a auditoria relaciona a modalidade de acesso aos Termos do Consumidor. Além disso, "saída do agente só avalia" não cobre as regras de D7, escritas pelo Claude.
- **Artefato:** [provenance-matrix.json](../research/frx_ai/audit/provenance-matrix.json).
- **Resultado correto:** o agente de v1–v6 é o Codex; os termos da OpenAI são o ponto de partida para esses rótulos; os da Anthropic, para P7, J7, C7, D7 e os antecedentes. Os resultados científicos não mudam. Afirmações originais preservadas.

### Pendência sobre os termos

**Não há conclusão jurídica neste diário.** A documentação oficial da Anthropic permite certos usos de saídas para treinar modelos não concorrentes e também traz restrições; falta esclarecer como isso se aplica à FRX AI e a estes dados.

| Fonte oficial | Consulta em 30/10/2026 |
|---|---|
| [OpenAI Terms of Use](https://openai.com/policies/row-terms-of-use/) | **Não lida**, HTTP 403 |
| [OpenAI Usage Policies](https://openai.com/policies/usage-policies/) | **Não lida**, HTTP 403 |
| [OpenAI Services Agreement](https://openai.com/policies/services-agreement/) | **Não lida**, HTTP 403 |
| [Codex com plano ChatGPT](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan) | **Não lida**, HTTP 403 |
| [Anthropic: Can I use my Outputs to train an AI model?](https://support.claude.com/en/articles/12326764-can-i-use-my-outputs-to-train-an-ai-model) | Lida, 16/03/2026 |
| [Anthropic Usage Policy](https://www.anthropic.com/legal/aup) | Lida; mostra vigência 12/11/2026; versão em vigor hoje não identificada |
| [Anthropic Consumer Terms](https://www.anthropic.com/legal/consumer-terms) | Lida, vigência 08/10/2025 |
| [Anthropic Commercial Terms](https://www.anthropic.com/legal/commercial-terms) | Lida, vigência 17/06/2025 |
| [Claude Code: legal e conformidade](https://code.claude.com/docs/en/legal-and-compliance) | Lida |

Trechos e tensões entre esses textos estão na [auditoria, seção 5](../research/frx_ai/audit/PROVENANCE-AUDIT.md). Questão aberta: [Q-012](#questoes).

### Próximo passo

[Plano de avaliação independente](../research/frx_ai/audit/INDEPENDENT-EVALUATION-PLAN.md), com critérios verificáveis. Plano futuro, ainda não executado.

### Decisões do responsável depois da auditoria (30/10/2026)

As seções acima estão preservadas. Estas decisões acrescentam, e não apagam, os registros e as erratas.

1. **Proveniência:** a auditoria corrige a conclusão anterior. Os dados usados nos treinos de v1–v6 foram produzidos pelo Codex, não pelo Claude. Modelos e registros históricos são preservados integralmente.
2. **Classificação:** se a FRX AI é ou não um modelo concorrente permanece **sem determinação jurídica**. Este diário não afirma que houve violação nem que os pesos estão liberados para distribuição. O responsável vai solicitar esclarecimento à OpenAI.
3. **Modelos:** pesos de v1–v6 ficam em ambiente de pesquisa. Sem distribuição e sem novos treinos com rótulos de proveniência contratualmente incerta, por enquanto. Avaliação interna e auditoria podem continuar.
4. **Benchmark:** piloto de 30 a 50 casos, com protocolo, critérios de concordância e gabarito separado. Ainda não há anotadores independentes confirmados, e resultados de agentes não são apresentados como validação humana. Preparado com [40 casos](../research/frx_ai/benchmark/PILOT-PROTOCOL.md).
5. **Logs:** inicialmente fixtures e logs públicos com uso permitido. Nenhum arquivo privado do computador ou de jogos reais sem autorização específica.

### Termos da OpenAI (conta ChatGPT Plus)

**Sem conclusão jurídica.** As páginas de openai.com e help.openai.com recusaram leitura automática (HTTP 403), e uma cópia de arquivo também não pôde ser lida. **O texto integral não foi lido diretamente neste diário.** Cada item indica a origem:

| Item | Registro | Origem |
|---|---|---|
| [Terms of Use](https://openai.com/policies/row-terms-of-use/) vigentes desde 01/01/2026 | Regem o uso individual. Restrição: "Use Output to develop models that compete with OpenAI" | Data informada pelo responsável. Frase e data vistas em resultado de busca restrita a openai.com, sem leitura da página |
| [Usage Policies](https://openai.com/policies/usage-policies/) | Atualização publicada em outubro de 2026, com entrada de 21/10/2026, trata de treinar outros modelos com modelos e saídas da OpenAI. **Só entra em vigor em 20/11/2026.** | Vigência informada pelo responsável. Confirmada por resultado de busca, com o [aviso da atualização](https://help.openai.com/en/articles/12092907-we-re-updating-our-usage-policies) e [o anúncio](https://openai.com/index/broad-access-through-consistent-standards/), sem leitura das páginas |
| [Services Agreement](https://openai.com/policies/services-agreement/) | Para API e clientes empresariais. O resultado de busca cita exceções para classificadores não distribuídos e fine-tuning. **Essas exceções não são aplicadas aqui a uma conta ChatGPT Plus.** | Resultado de busca, sem leitura da página |

**A confirmar pelo responsável, no navegador:**

- O resultado de busca rotula a versão de 01/01/2026 como "Previous version". Isso pode ser um erro do buscador ou indicar uma versão mais nova.
- A redação exata das Usage Policies a partir de 20/11/2026. Um termo citado por um resumo de busca não foi confirmado por outro, e por isso não é citado aqui.
- Se há termos específicos do Codex. A busca não encontrou nenhum.

A errata E-001 continua válida, e a pendência é rastreada em [Q-012](#questoes).

FRX AI — Specialized Linux Gaming Intelligence.

FRX — Play. Fix. Repeat.
