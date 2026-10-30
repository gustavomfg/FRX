# FRX AI — investigação multidisciplinar, 30/10/2026

**Resultado: a recuperação treinada anteriormente resistiu a uma nova avaliação; os novos modelos de linguagem e de diagnóstico não estão aprovados para conclusões automáticas.** Entregamos um protótipo local de investigação, com fontes, observações literais, restrições e memória contextual. Sua composição completa ainda não é uma inteligência de diagnóstico validada.

## Pesquisa e desenho experimental

Foram investigados NLP, recuperação, AIOps, grafos, diagnóstico bayesiano, seleção de verificações, restrições e inferência causal. [DocRED](https://aclanthology.org/P19-1074/), [PURE](https://aclanthology.org/2021.naacl-main.5/), [ERICA](https://aclanthology.org/2021.acl-long.260/) e [SetFit](https://arxiv.org/abs/2209.11055) fundamentam a separação entre propostas, relações e representações aprendidas. As implementações públicas foram verificadas; não executamos código desses repositórios.

[Efficient Document-level Event Relation Extraction](https://aclanthology.org/2025.repl4nlp-1.7/) propõe recuperar pares antes de classificá-los. [DEEIA](https://aclanthology.org/2024.findings-acl.564/) investiga argumentos de múltiplos eventos. Não são evidências de que comentários de Proton possam ser interpretados corretamente com os nossos rótulos atuais.

[AutoTSG](https://www.microsoft.com/en-us/research/publication/autotsg-learning-and-synthesis-for-incident-troubleshooting/), [SuperDRI](https://www.microsoft.com/en-us/research/publication/superdri-graph-guided-autonomous-troubleshooting-for-cloud-databases/) e [StepFly](https://www.microsoft.com/en-us/research/publication/stepfly-agentic-troubleshooting-guide-automation-for-incident-diagnosis/) sugerem separar conhecimento, investigação e execução. O código público do StepFly usa infraestrutura e APIs de LLM que não adotamos. Implementamos uma representação pequena em memória, sem executor.

[Decision-Theoretic Troubleshooting](https://www.microsoft.com/en-us/research/publication/decision-theoretic-troubleshooting/) fundamentou a seleção por informação/custo. [SafeOpt](https://proceedings.mlr.press/v37/sui15.html) foi investigado, mas não aplicado: faltam uma função de segurança e observações reais apropriadas. Um [estudo recente de grafos versus modelos planos em RCA](https://arxiv.org/abs/2609.27069) motivou controles com as mesmas variáveis; ele é um preprint, não uma garantia para o FRX. A [bibliografia comentada](../research/frx_ai/v3/LITERATURE.md) registra contribuições, código e decisões.

Realizamos **24 configurações com parâmetros aprendidos**:

| Família | Configurações | Hipótese / resultado |
| --- | ---: | --- |
| Cabeças separadas de modalidade e observação | 12 | TF-IDF ou embeddings originais/FRX; separar tarefas não resolveu a atribuição nem o ruído das propostas. |
| Ranking pairwise | 6 | Combinar semântica, léxico e janelas; não superou a alternativa simples no desenvolvimento. |
| Likelihoods por contexto/suavização | 3 | Diagnóstico e ganho de informação em simulação; confiança depende fortemente das hipóteses. |
| Independência, duplicatas aprendidas e árvore aprendida | 3 | Dependência entre observações; árvore mais complexa não demonstrou vantagem suficiente. |

Baselines de BM25, TF-IDF, encoders, fusão de rankings e janelas são comparações adicionais. Não contamos cada teste de software como experimento científico. Nesta rodada não houve novo fine-tuning de encoder: reutilizamos os pesos neurais da v2 e treinamos cabeças lineares, rankers, probabilidades e estrutura. Os resultados inferiores foram preservados.

## Dados e prevenção de vazamento

O treino de linguagem usa 209 relações antigas; o desenvolvimento usa 96 unidades do antigo teste v2, agora explicitamente conhecido. O ranking usa 183 comentários de treino/desenvolvimento antigos e esse mesmo conjunto conhecido para seleção. Jogos de treino e de desenvolvimento permanecem separados.

O novo teste reservou **95 comentários de seis discussões**, sem seleção por palavras de sucesso: Resident Evil Village, Helldivers 2, Final Fantasy VII Remake Intergrade, Detroit: Become Human, Monster Hunter Wilds e Kingdom Come: Deliverance II. As identidades foram selecionadas por hash antes de ler os textos. Houve 16 comentários por jogo, exceto FFVII, com 15 elegíveis. Código, modelos e escolhas foram congelados antes da anotação final; as previsões só foram executadas depois dos rótulos.

**Os rótulos são provisórios, produzidos pelo agente, sem revisão humana independente.** Há hashes, URLs, atribuição por título/AppID, offsets e notas de ambiguidade. Uma fila de revisão esconde os rótulos. Comentários completos ficam fora do Git. Fontes editadas não podem reutilizar offsets silenciosamente.

Este teste avalia unidades propostas automaticamente; não mede recall global de extração de eventos. O treino usa spans antigos anotados e a avaliação inclui propostas ruins e campos de templates. Isso mistura mudança de domínio e erro de proposta. Não apresentamos essa tarefa como uma comparação direta com os quatro campos conjuntos da v2.

## Linguagem: resultado insuficiente

| Componente selecionado no desenvolvimento | F1 macro desenvolvimento | F1 macro final | Seleção com score ≥0,8 |
| --- | ---: | ---: | --- |
| Modalidade, FRX + regressão logística | 0,466 | 0,310 | 25/63 corretos; 66,3% de cobertura |
| Observação, TF-IDF + regressão logística | 0,267 | 0,204 | Zero afirmações selecionadas |

Segundo os rótulos provisórios, houve **33 falsas classificações de tentativa executada** no primeiro componente. No segundo, precisão seletiva é indefinida, pois a cobertura é zero. Nenhum atingiu a meta de 90% com cobertura útil; seus scores não são probabilidades de segurança ou sucesso.

A análise dos textos mostrou problemas concretos: “updates” no template vira ação; “enabled” em inicialização de hardware vira intervenção; uma mensagem sobre Wi-Fi é escolhida como resultado do jogo; um agradecimento pode ser selecionado como observação; um resultado de outra versão ou objetivo é associado ao procedimento errado. Separar cabeças não elimina esses problemas. Não corrigimos o modelo usando este novo teste nem o reutilizamos como independente para outro ajuste.

## Recuperação: o componente anterior continua promissor

O teste tem oito consultas de sintomas por jogo: **31 respondíveis e 17 sem fonte rotulada relevante**. As consultas são templates em inglês, não consultas reais de jogadores. A busca é dentro dos comentários fornecidos para o mesmo AppID, não em toda a internet.

| Método | P@3 | Recall@5 | NDCG@5 | MRR |
| --- | ---: | ---: | ---: | ---: |
| BM25 | 0,495 | 0,525 | 0,568 | 0,725 |
| MiniLM original INT8 | 0,591 | 0,622 | 0,725 | 0,919 |
| FRX v2 preservado INT8 | **0,634** | 0,682 | **0,794** | **0,945** |
| Novo candidato escolhido: FRX por janelas | 0,634 | **0,685** | 0,772 | 0,898 |

O challenger por janelas tinha NDCG 0,740 no desenvolvimento, mas não sustentou a vantagem. O novo teste fortalece a evidência a favor do **encoder já treinado na v2**, sem transformar essa rodada em alegação de um modelo neural novo superior.

Bootstrap descritivo por jogo, com 4.000 reamostragens, estimou ganho de NDCG do FRX v2 contra o original de **0,069**, intervalo de 95% **[0,028; 0,124]**. Os intervalos de P@3, Recall@5 e MRR incluem zero. São apenas seis grupos de jogos, rótulos provisórios e pequenos conjuntos de candidatos; não é validação populacional nem eficácia de correções.

As 17 consultas sem resposta ficam registradas separadamente, fora das médias acima. A ausência de resposta ainda não é detectada de forma confiável: o sistema pode ordenar textos sem que exista solução relevante. Por isso, ranking não gera uma recomendação aprovada.

## Investigação: capacidades efetivas e limites

O protótipo combina recuperação com operações determinísticas identificadas como tais:

- Observações literais e limitadas de erros de carregamento Wine e falhas de dispositivo/fila gráfica. A presença de `PhysX.dll` ou uma GPU NVIDIA não gera diagnóstico PhysX.
- Hipóteses investigáveis com fontes, suportes e limites. “Biblioteca não encontrada” não identifica automaticamente o redistribuível correto; perda de dispositivo não prova culpa do driver.
- Requisitos com estados compatível, incompatível e desconhecido. Fontes de outro jogo ou ambiente explicitamente conflitante são excluídas; metadados ausentes permanecem desconhecidos.
- Fatos documentados separados de relatos e observações. Afirmações contraditórias sobre tecnologias continuam conflitantes. URLs e fatos enviados pelo chamador não são autenticados remotamente pela inferência.
- Histórico persistente de tentativas informado pelo chamador, separado por AppID/GPU/driver/Proton. Uma falha não vira proibição global; mudanças de ambiente impedem reutilizar o resultado anterior. Nenhuma previsão neural entra nesse histórico.

A correspondência de ambiente compara identificadores textuais de forma conservadora. Nomes equivalentes escritos de outra maneira podem causar exclusões; isso não constitui inteligência de compatibilidade aprendida por hardware.

O protótipo foi executado sobre **oito demonstrações históricas**, em quatro jogos, contendo logs públicos de bibliotecas ausentes e erros gráficos. Esses exemplos foram encontrados pelo próprio parser e inspecionados: **não são um teste independente de sensibilidade ou precisão diagnóstica**. No exemplo de The Last of Us Part I, preserva `AVRT.dll`, a fonte e o erro observado, e pede verificar origem/arquitetura/versão. Não inventa um download.

Os formatos cobertos são estreitos. Erros de função não implementada, hangs específicos do kernel, logs anexados e dependências não explícitas ainda podem resultar em informação insuficiente. O sistema não declara causas confirmadas nem extrai procedimentos completos confiáveis de texto livre. Ordem textual de tentativas também não é prova de ordem temporal ou causalidade.

Após o teste do challenger, montamos `case_review.py` usando a baseline v2 preservada e memória local; as cabeças reprovadas ficam disponíveis apenas com flag experimental. Essa montagem posterior não altera o congelamento nem os resultados, mas **o fluxo combinado ainda precisa de uma avaliação diagnóstica independente**.

## Aprendizado probabilístico: simulações separadas

Um simulador explícito define quatro classes de falha, sete verificações binárias, ruído e custos abstratos. Priors e likelihoods são estimados em 1.200 incidentes sintéticos. Os números não descrevem jogos reais.

O ensaio inicial mostrou que contar um sensor duplicado como independente reduz custo aparente, mas aumenta diagnósticos errados com confiança alta. Um segundo protocolo reservou novos seeds finais e comparou Bayes ingênuo, grupos de duplicatas aprendidos e uma árvore aprendida por informação mútua condicional. A inferência enumera 128 estados; não exige banco de grafos.

No novo teste sintético de 600 casos:

| Modelo / política | Acurácia | Custo médio abstrato | Diagnósticos errados com posterior ≥0,9 |
| --- | ---: | ---: | ---: |
| Bayes ingênuo, ordem fixa | 84,3% | 4,621 | 31 |
| Bayes ingênuo, ganho de informação | 82,5% | 3,954 | 52 |
| Duplicatas aprendidas, ganho de informação | 85,2% | 4,286 | 33 |
| Árvore aprendida, ganho de informação | 83,8% | 4,312 | 35 |

A política selecionada **antes desse teste**, pelas restrições no desenvolvimento, foi a ordem fixa ingênua. Não escolhemos retrospectivamente o maior número da tabela. O controle com duplicatas esclarece o mecanismo, mas a árvore não demonstrou vantagem necessária para adoção.

Sob novo ruído, a acurácia da política selecionada caiu para **56,8%**, com 98 diagnósticos errados confiantes em 600. A referência apenas pelo prior acertou 30,2% no teste original. Esses resultados explicam por que não oferecemos probabilidades de diagnóstico para casos reais: confiança exige um modelo de observação adequado, dependências corretas e cobertura das causas possíveis. Nenhum modelo simulado foi conectado à inferência de jogos.

## Execução, custos e reprodução

O [README do laboratório](../research/frx_ai/v3/README.md) contém os comandos. O caminho padrão usa o encoder INT8 anterior, aproximadamente **22,3 MiB com tokenizer**; os dois novos classificadores experimentais somam cerca de 90 KiB e ficam fora do Git. Não foram baixados modelos novos. Não há API de inferência, Ollama, PyTorch em runtime ou alteração dos instaladores.

Na CPU Ryzen 5 4600G, quatro threads, a montagem padrão apresentou mediana quente de **17 ms**, p95 de **24 ms**, em 24 consultas repetindo oito fontes conhecidas. Cinco processos novos tiveram mediana de **0,987 s**. O processo replicado registrou cerca de **216 MiB** de pico no `/proc`; o lançamento inicial havia registrado 830 MiB via `ru_maxrss`, também preservado. Essas medições pequenas não são benchmark de produção ou de jogos.

O orçamento fixado foi de 45 minutos acumulados de ajuste/avaliação, 6 GiB de RAM, 1 GiB de downloads e 2 GiB de cache novo. Os estágios temporizados de ajuste/avaliação somam cerca de **9 segundos**, excluindo extração de embeddings, coleta, anotação e avaliação final. Não apresentamos esse número como tempo total de pesquisa. O cache novo ficou próximo de 12 MiB, reutilizando os artefatos anteriores.

Passaram **16 contratos novos e os 21 anteriores**. Houve execução do CLI com fonte real, verificação dos arquivos congelados, reconstrução dos textos por hash e teste em três processos separados: falha lembrada após reiniciar e não transportada para outro driver. Testes de contratos não provam qualidade linguística ou diagnóstica.

## Entrega e pendências

Arquivos desta rodada estão em `research/frx_ai/v3/` e neste relatório: modelos experimentais, ranking, investigação, memória, simulações, avaliação, ferramentas de reprodução, protocolos e snapshots. Todos os arquivos preexistentes foram comparados com seus hashes iniciais e preservados. Não houve alteração da interface, Complete Experience Pack, Fix Engine, Steam, prefixos, jogos ou configurações do sistema. Nenhum commit, tag ou push foi feito.

Próximos obstáculos: revisão humana independente; propostas de eventos e objetivos realmente anotadas; distinguir observação de atribuição causal; recuperar contexto de threads/edições; detectar perguntas sem resposta; ampliar logs com avaliação independente; autenticar fontes upstream; aprender compatibilidade sem transportar resultados entre ambientes. A recuperação é o componente mais sustentado. A investigação estruturada é utilizável como ferramenta de pesquisa, mas a interpretação automática e o diagnóstico aprendido continuam abertos.
