# FRX AI v5 — resultados da pesquisa

30/10/2026. Escopo: somente `research/frx_ai/v5/`. Aplicação, Steam, jogos, prefixos, empacotamento e laboratórios anteriores preservados. Sem commits, tags ou push.

## Perguntas e experimentos

Foram realizados **18 ajustes exploratórios**: seis execuções de MiniLM e doze classificadores de associação. A auditoria de reprodução acrescentou **dois controles MiniLM**, totalizando 20 execuções de treinamento. Uma execução inicial repete deliberadamente o mesmo ajuste para comparar seleção de checkpoint; não representa uma arquitetura diferente. Grades de limiares, revisões do gerador, controles de recuperação e casos de replay são contabilizados separadamente em [experiments.json](snapshot/experiments.json).

1. **Dados e unidade de anotação:** 26 unidades antigas sinalizadas para revisão, sem alterar os rótulos. Acrescentados 14 relatos reais/36 eventos ao treino, anotados novamente pelo agente. Casos incluem negação parcial, resultados conjuntos, declarações riscadas, instruções versus ações, diagnóstico versus correção e mudança de ambiente. Não houve revisão humana. A auditoria é uma melhoria de rastreabilidade, não comprovação de que os novos rótulos estão corretos.
2. **Decomposição sintática:** comparar v4, verificação lexical, sintaxe inicial e reconstrução por trigger, argumento, modalidade, observação e grupo. Sintaxe pré-treinada ajuda a separar a negação do resultado da execução da tentativa. A seleção final usa Stanza; spaCy ficou como controle menor.
3. **Verificação aprendida:** MiniLM com quatro afirmações binárias, alvo marcado e treinamento das duas últimas camadas. Controles: somente dados reais; +120 contrastes sintéticos; parágrafo versus frase; +14 atributos sintáticos; seleção orientada a cobertura; +dados auditados. O melhor checkpoint conservador aceitou 9/9 execuções em spans fornecidos no desenvolvimento, amostra pequena e sem medir detecção. O controle orientado a cobertura chegou a 19/29; com os dados adicionais, 24/41. Não adotados. Selecionar apenas pelo custo de falsos positivos também favorecia rejeitar tudo.
4. **Relações aprendidas:** regressão logística e árvore pequena, com grupos completos, exemplos reais, contrastes sintéticos e ablação de dependências. Três estágios do gerador/dados, quatro ajustes por estágio. Nenhum atingiu simultaneamente o critério predefinido de pelo menos quatro relações corretas e precisão de 80% no desenvolvimento. A geração não propôs o grupo correto em diversas observações; aumentar o classificador não corrige esse erro anterior. Não adotados.
5. **Recuperação de incidentes:** comentário completo, resumo literal estruturado e média dos escores, mantendo o mesmo FRX v2. Resumos perderam informações relevantes; não substituímos o índice de comentários.
6. **Investigação e memória:** snapshots imutáveis, verificações reveladas por cache, rótulos de avaliação separados da entrada, histórico contextual e identidade completa da ação/artefato. Sem comandos de sistema. Não ajustamos probabilidades bayesianas: faltam incidentes independentes com causas e probabilidades de observações verificadas.

**Falha de reprodução encontrada:** A5 antigo registrou 91 eventos, apesar da configuração prever os 127 eventos reais com os exemplos adicionais. Não há hash dos inputs daquela execução para certificar sua composição apenas pelos metadados. O controle em diretório novo confirmou127, mas mudou as curvas de treino. Um teste isolado demonstrou que a inicialização do parser altera o RNG do Torch somente com cache frio. Portanto, A1–A5 não isolam perfeitamente o efeito da arquitetura/dados. `A6-seed-control` reinicializa a semente depois dos atributos; o código também passa a registrar hashes das unidades e tensores em ajustes futuros. Preservamos os registros antigos e a errata; esses modelos continuam desativados. A reprodução **da inferência selecionada** foi exata nos cinco comparadores e não depende desses ajustes experimentais.

## Avaliação prospectiva

Treino conhecido: 69 unidades/91 eventos, mais os 14 casos adicionais. Desenvolvimento conhecido: 57 unidades/64 eventos de cinco jogos; o antigo teste v4 virou desenvolvimento explícito. Foram reservados 72 trechos de seis discussões/AppIDs ausentes desses conjuntos. Código, modelos e seleção foram congelados às **20:01:22 UTC**, antes de ler os corpos reservados. Anotações completas foram travadas antes das previsões. [Protocolo](PROTOCOL.md), [congelamento](snapshot/frozen.json), [rótulos](snapshot/final-audited.json), [resultado integral](snapshot/final-results.json).

| Comparador | Execuções corretas/afirmadas | Recall de 52 | Falsas execuções | Relações corretas/propostas | Falsos sucessos |
|---|---:|---:|---:|---:|---:|
| v4 conservador | 14/14 | 26,9% | 0 | 1/1 | 0 |
| Lexical v4 | 18/20 | 34,6% | 2 | 0/0 | 0 |
| Sintaxe inicial | 22/23 | 42,3% | 1 | 4/8 | 3 |
| v5 selecionado | **28/32** | **53,8%** | **4** | **6/9** | **1** |
| spaCy, controle | 28/31 | 53,8% | 3 | 6/9 | 1 |

O ganho de recall v5–v4 teve intervalo de bootstrap pareado por jogo de **+16,2 a +37,1 pontos percentuais**. São apenas seis grupos. Precisão de execução v5: **87,5%**, Wilson descritivo de **71,9–95,0%**. Não atingimos a meta de qualidade para afirmações automaticamente aceitas. A precisão conjunta de trigger/estado/direção foi 61,4%; acurácia de direção entre triggers correspondentes, 74,7%. Evidência literal não garante interpretação correta.

**Refutação importante:** duas das seis relações linguisticamente correspondentes pertencem a outros jogos mencionados na discussão de Spider-Man. Os AppIDs das discussões são novos; isso não significa que toda entidade mencionada seja inédita. Excluindo as duas execuções explicitamente de outros jogos: v5 **26/30**, recall **26/50**, versus v4 **12/12**, recall **12/50**. Somente quatro das seis relações corretas pertencem ao jogo da discussão, e nenhuma comprova efeito no ambiente atual. [Auditoria posterior](snapshot/posthoc-review.json).

Dois de três grupos conjuntos propostos foram correspondências corretas: substituição de distribuição e combinação de mods/ajuste de mitigação. Isso demonstra alguma reconstrução de procedimentos relatados, sem aprovação dessas operações ou atribuição causal individual. Recall global de associações: **6/32**, ainda baixo. Os quatro erros de execução envolveram configuração preexistente, instalação já existente e tentativa condicional. O falso sucesso deu crédito conjunto a uma configuração anterior e à remoção que o autor apontava como solução.

Depois da avaliação, `run.py` passou a deixar **toda atribuição de evento a jogo/ambiente como não verificada**. Esse bloqueio de apresentação não altera o extrator congelado e não é contabilizado como avanço científico. Não mudamos a seleção para spaCy com base no teste. Uma nova seleção exige uma coorte independente.

## Recuperação e diagnóstico

No replay conhecido do v3, o FRX v2 reproduziu NDCG@5 **0,794**, MRR **0,945**, P@3 **0,634**, Recall@5 **0,682**; o MiniLM original obteve NDCG@5 **0,725**. Não é uma validação nova. Resumos estruturados caíram para **0,668** de NDCG; o híbrido chegou a **0,767**. Preservamos o recuperador original. [Comparação](snapshot/incident-retrieval.json).

Nove cenários históricos/controlados produziram dez etapas de replay; a verificação esperada ficou no topo em 10/10. Esse resultado pequeno e selecionado **não demonstra diagnóstico aprendido**: o seletor reaproveita padrões de logs do v3/v4. A falha de função aponta a verificação do componente exato, não um pacote inventado; o caso sem sinais pede log e mantém causa desconhecida. A memória persistiu após reinício e isolou mudança de driver. IDs completos distinguem uma nova versão/hash do artefato anteriormente malsucedido. Testes cobrem outros jogos, GPUs, Proton, builds e configurações. [Replay](snapshot/incident-replay.json).

## Recursos, reprodução e limite atual

CPU Ryzen 5 4600G, quatro threads, sem GPU. Ajustes cronometrados: **559 s exploratórios +240 s nos controles de reprodução**, sem incluir coleta, anotação e toda a preparação. Pesos novos baixados: aproximadamente 244 MiB. Cache total, incluindo cópias para reprodução: cerca de **1,2 GiB**; orçamento de 2 GiB. Os limites foram práticos, sem isolamento por cgroup. [Controles de reprodução](snapshot/replication-controls.json) e [auditoria do RNG](snapshot/seed-audit.json) preservam os resultados negativos.

Modelo selecionado: 242.912.160 bytes de sintaxe + 22.950.006 bytes do recuperador. Processo completo: **797 MiB RSS**, primeira requisição **3,46 s**, mediana aquecida sem cache **142 ms**, P95 **348 ms**. Sintaxe isolada: 125 ms; o controle spaCy usa 15,2 MB instalados, 336 MiB RSS e mediana de 6,4 ms. Tempos de cache foram medidos separadamente. Isso não mede integração Rust nem tamanho de instalador. [Benchmark](snapshot/benchmark-runtime.json).

Foram aprovados 14 testes do laboratório, compilação Python, verificação dos artefatos congelados, execução JSONL dos exemplos e reconstrução offline das fontes por hashes. Não executamos a suíte do aplicativo: seu código não foi alterado. Os 480 estados de arquivos presentes no início foram preservados; os resultados da checagem ficam em `snapshot/preservation.json`.

O protótipo é utilizável para **investigação assistida com evidências**, ainda sem conclusões aprovadas automaticamente. A próxima hipótese é aprender e verificar primeiro a atribuição **ator/jogo/ambiente** e as transições **condicional/preexistente → tentativa executada**, antes de ampliar a associação de resultados. Precisaremos de revisão humana de um conjunto pequeno e de uma nova avaliação independente. Nenhuma previsão foi conectada ao Fix Engine.

## Arquivos entregues

Todos são novos e ficam neste diretório. Nenhum arquivo anterior foi alterado.

| Grupo | Arquivos principais |
|---|---|
| Protótipo e memória | `run.py`, `lab.py`, `incidents.py` |
| Decomposição e reconstrução | `events.py`, `relations.py`, `syntax.py`, `compact.py`, `syntax_control.py` |
| Modelos e ajustes experimentais | `atomic.py`, `train_atomic.py`, `training_data.py`, `linker.py`, `train_linker.py`, `relation_data.py` |
| Avaliação e controles | `assess.py`, `study.py`, `initial_study.py`, `final_evaluate.py`, `posthoc.py`, `retrieval_study.py`, `replay_study.py` |
| Reprodutibilidade | `reserve.py`, `seal.py`, `hydrate.py`, `publish.py`, `make_examples.py`, `setup_syntax.py`, `setup_compact.py`, `common.py` |
| Validação e documentação | benchmarks, dois arquivos de testes, requisitos, protocolo, contrato de anotação, README, este relatório e referências |
| Artefatos pequenos | `snapshot/`: métricas, rótulos sem corpos, hashes, seleção e fila de revisão humana |

Os comandos estão no [README](README.md). Pesos, checkpoints, corpos de comentários, logs e memórias de demonstração permanecem no cache externo, fora do Git. O `.gitignore` existente do laboratório já os exclui.
