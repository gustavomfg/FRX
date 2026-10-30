# RXAI — FRX AI Research Journal
### Registro Científico de Pesquisa, Experimentação e Desenvolvimento

**Edição:** 1 · **Criado e atualizado:** 30/10/2026 · **Cobertura:** v1–v6.

RXAI é o diário científico da FRX AI. Não designa um modelo, uma arquitetura ou
uma versão do aplicativo. Este documento consolida os registros locais e aponta
para os artefatos detalhados; não substitui os relatórios nem os congelamentos.

**Conclusão atual:** recuperação especializada apresenta evidências favoráveis;
interpretação de procedimentos e diagnóstico autônomo continuam experimentais.
Não há validação humana independente documentada dos rótulos de avaliação.

## Navegação

[I. Identidade](#identidade) · [II. Origem](#origem) ·
[III. Problema e critérios](#problema) · [IV. Laboratórios](#historico) ·
[V. Linha do tempo](#linha-do-tempo) · [VI. Estado atual](#estado-atual) ·
[VII. Descobertas](#descobertas) · [VIII. Questões abertas](#questoes) ·
[IX. Referências](#referencias) · [X. Reprodução e lacunas](#reproducibilidade) ·
[XI. Manutenção](#manutencao) · [XII. Changelog](#changelog).

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

P6-M e P6-C somam 96 trechos, 74 execuções e 70 relações. O agregado é descritivo;
o enriquecimento e as citações repetidas impedem tratá-lo como prevalência geral.
Não há curva única de qualidade v1→v6: mudam tarefas, unidades e conjuntos.

<a id="historico"></a>
## IV. Histórico científico — v1 a v6

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

<a id="estado-atual"></a>
## VI. Estado da pesquisa após o v6

| Capacidade | Evidência favorável | Estado permitido |
|---|---|---|
| Recuperação especializada | P2-I e nova coorte P3 favoreceram o recuperador v2 em NDCG; replays v4/v5 o preservaram | Componente promissor de busca; relevância não confirma eficácia |
| Identificação de alguns eventos | Comparações pareadas P5/P6-M recuperaram mais execuções | Experimental; falsos positivos, identidade e cobertura persistem |
| Memória contextual | Contratos e processos separados demonstraram persistência/isolamento de tentativas do chamador | Ferramenta experimental; não aprende causalidade |
| Investigação com logs explícitos | Perguntas pertinentes em exemplos históricos e replays limitados | Assistência por regras; causa desconhecida permanece desconhecida |
| Inferência local compacta | v6 sem Torch, artefatos ativos ≈38,7 MB; 23 contratos e CLI com 11 casos | Viabilidade técnica; não aprovação linguística ou de produção |

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
[v5](../research/frx_ai/v5/LITERATURE.md) e [v6](../research/frx_ai/v6/LITERATURE.md)
preservam as demais leituras e os limites de disponibilidade de código.

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

A disponibilidade de código descrita nos relatórios é histórica: falha de acesso
ou ausência de link na consulta não prova inexistência permanente. Licenças de
modelos, dados de treino e comentários são questões distintas; acesso público e
checksum não autorizam redistribuição indiscriminada nem atestam segurança de um patch.

<a id="reproducibilidade"></a>
## X. Reprodutibilidade, disponibilidade e lacunas

### Como localizar os procedimentos

Este diário não reedita comandos de treino como se tivessem sido reexecutados.
As entradas abaixo foram conferidas por leitura dos scripts/READMEs; esta
consolidação é documental e não realizou novos treinamentos ou avaliações.

| Etapa | Guia operacional | Inferência | Avaliação/treino e dependências externas |
|---|---|---|---|
| v1 | [README raiz](../research/frx_ai/README.md) | `prototype.py`: Python, ONNX Runtime, tokenizers e pesos MiniLM/NLI | Corpus anotado 100/50/80 e cabeças no cache externo; coletor não recria sozinho os rótulos |
| v2 | [README v2](../research/frx_ai/v2/README.md) | `prototype.py`, modelos ONNX por objetivo/recuperação | `data.py`, `replay.py` e scripts de treino; modelos, tokenizers, corpus por hash e checkpoints externos; PyTorch no treino |
| v3 | [README v3](../research/frx_ai/v3/README.md) | `case_review.py`, v2 preservado, memória opcional | `reproduce.py` e `hydrate_final.py`; exige artefatos/dados v2 e textos originais para conferir offsets |
| v4 | [README v4](../research/frx_ai/v4/README.md) | `run.py`, detector ONNX e recuperador v2 | `hydrate.py`, `reproduce.py`, v2/v3 e cache de corpos; requisitos de treino separados |
| v5 | [README v5](../research/frx_ai/v5/README.md) | `run.py`, Stanza e FRX v2; spaCy como controle | `hydrate.py`, `train_atomic.py`, `train_linker.py`; v4, modelos/manifests sintáticos, dados e checkpoints externos |
| v6 | [README v6](../research/frx_ai/v6/README.md) | `investigate.py`, spaCy, ONNX v2 e JSON do verificador; sem Torch no caminho selecionado | `rehydrate.py`, `final.py`, `learning.py`, `neural.py`; caches antigos, corpos, tokenizers e pesos externos |

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
| Alcance da consolidação Git | Na inspeção de 30/10/2026, `v5/` e `v6/` estão presentes, mas não rastreados. Seus arquivos não entram no commit documental solicitado. Links existem neste working tree; um clone só desse commit pode não conter tais artefatos |

A última limitação de versionamento não autoriza incluir laboratórios, modelos
ou dados em um commit de documentação. Quando forem versionados separadamente,
registrar sua disponibilidade no changelog, preservando a síntese já publicada.

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

FRX AI — Specialized Linux Gaming Intelligence.

FRX — Play. Fix. Repeat.
