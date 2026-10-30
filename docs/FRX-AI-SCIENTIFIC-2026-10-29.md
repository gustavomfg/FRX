# FRX AI — pesquisa científica de relações e recuperação

Data: 29/10/2026. **Resultado: direção promissora para ordenar evidências; interpretação automática de correções reprovada.** Nenhuma previsão foi integrada ao aplicativo ou ao Fix Engine.

## Método e orçamento

A rodada realizou **42 configurações com ajuste de parâmetros aprendidos**: 18 experimentos neurais, 12 baselines TF-IDF/logística, dez simulações de aquisição ativa e dois treinos de recuperação contrastiva. E09 usa spans de referência como diagnóstico e foi excluído da seleção; E18 avalia uma tarefa diferente, condicionada ao par ação/observação. Exportação, calibração e auditorias adicionais não foram contadas como novos treinos. O [registro dos experimentos](../research/frx_ai/v2/snapshot/experiments.json) preserva resultados e duração, inclusive abordagens inferiores.

O orçamento anterior aos experimentos foi de 90 minutos acumulados de execução experimental, quatro threads de CPU, alvo de 6 GiB de RAM, 2 GiB de downloads e 4 GiB de cache novo. Foram registrados 4.821 segundos, aproximadamente 80 minutos, de experimentos temporizados. Preparação, anotação e análise ficam fora desse total. Usamos Ryzen 5 4600G; treinamento em CPU, sem instalar CUDA. O cache ficou próximo de 3,3 GiB e os treinos abaixo de aproximadamente 2,2 GiB de RAM.

## Fundamentação e hipóteses

[DocRED](https://aclanthology.org/P19-1074/) motivou preservar contexto e evidências entre frases; [PURE](https://aclanthology.org/2021.naacl-main.5/) motivou separar propostas de entidades da classificação de relações. [ERICA](https://aclanthology.org/2021.acl-long.260/) e [SetFit](https://arxiv.org/abs/2209.11055) fundamentaram contrastes e adaptação de representações compactas. São inspirações experimentais, não reproduções desses sistemas.

[Snorkel](https://link.springer.com/article/10.1007/s00778-019-00552-1) orientou manter supervisão fraca separada de referência. [Temperature scaling](https://proceedings.mlr.press/v70/guo17a.html), classificação seletiva e conjuntos conformais foram investigados sem assumir que confiança elevada garante associação correta. Extração de opinião por alvo, relações temporais, BADGE e Confident Learning também foram consultados; decisões e equações estão em [LITERATURE.md](../research/frx_ai/v2/LITERATURE.md).

Hipóteses: spans explícitos seriam superiores ao sentimento do comentário inteiro; fine-tuning e contrastes ajudariam a reconhecer direção e modalidade; separar objetivos evitaria atribuir a cada ação o resultado de uma combinação; recuperação semântica poderia melhorar mesmo sem resolver interpretação.

## Dados e independência

O corpus anotado desta rodada usa comentários públicos de **ValveSoftware/Proton**, com URL, AppID verificável pela discussão, texto original em cache, hash, datas e offsets de evidência. Documentação técnica foi consultada, mas não apresentada como se fosse experiência de usuário. A coleta não acessou a Steam.

| Conjunto principal | Relações anotadas | Jogos |
| --- | ---: | ---: |
| Treino ampliado | 167 | 13 |
| Desenvolvimento | 42 | 4 |
| Calibração | 18 | 1 |
| Teste final novo | 96 | 8 |

Treinos iniciais usaram 87 e depois 113 relações; o modelo principal escolhido usa 113 reais e 320 exemplos sintéticos de treino. Há 53 exemplos de supervisão fraca separados. A tarefa por objetivo possui 183 relações de treino, 44 de desenvolvimento e 20 de calibração. Esses números são relações, não necessariamente documentos distintos.

**Todos os rótulos são anotações provisórias deste agente de pesquisa, sem revisão humana independente.** Não houve estimativa de concordância entre anotadores. Exemplos ambíguos permanecem desconhecidos; rótulos fracos e sintéticos não são referência de avaliação.

O teste final reservou previamente DOOM Eternal, Sekiro, Starfield, Jedi: Fallen Order, DEATHLOOP, No Man's Sky, Final Fantasy XVI e The Last of Us Part I, com 12 propostas por jogo. Treino, desenvolvimento e teste são separados por jogo/discussão, com deduplicação. O teste antigo passou a ser material conhecido. Modelos, limiares, código e entradas foram selados antes da anotação final; nenhuma previsão foi vista durante essa anotação. Uma primeira tentativa de selo foi substituída antes de acessar o teste, após um teste de contrato detectar problema de reconstrução parcial dos dados; modelos e limiares permaneceram iguais. A justificativa está no snapshot.

Os resultados são condicionados a propostas determinísticas de ações. **Não medimos recall global de extração de entidades.** A amostra final tem muitos falsos candidatos, como “updated my comment”, e referências a outro jogo ou Windows dentro de uma discussão Linux. Isso é uma limitação real do pipeline, não uma razão para retirar exemplos difíceis depois da avaliação.

## O que os experimentos ensinaram

O encoder MiniLM foi efetivamente ajustado, com cabeças de resultado, direção, modalidade e agrupamento. A perda combina essas tarefas e contraste supervisionado; as propostas e validações de contexto permanecem determinísticas. O melhor modelo principal no desenvolvimento foi **E07-full-contrast005**, macro F1 de resultado 0,581 e acerto integral de quatro campos de 35,7%.

Nesse treino, a perda caiu de 2,523 na primeira época para 0,259 na sexta, cujo checkpoint foi escolhido. Na sétima caiu novamente para 0,173, mas o F1 de desenvolvimento piorou para 0,462: reduzir a perda de treino não garantiu generalização. Os históricos estão em `snapshot/training-traces.json`.

Cabeças sobre embeddings congelados e treino apenas real foram fracos. Dobrar os sintéticos para 640 piorou a generalização de desenvolvimento. Currículo, exemplos difíceis adicionais, balanceamento e 53 rótulos fracos não trouxeram ganho confiável. A seleção por incerteza/diversidade superou aquisição aleatória em aproximadamente 0,027 de macro F1 médio, mas em apenas duas sementes: sinal exploratório, não conclusão robusta.

A tarefa E18 relaciona ação e trecho de observação por objetivo. Ela permite representar “parou de travar, mas ficou lento” em relações distintas, preservando o grupo de ações e deixando a causa individual desconhecida. Isso melhorou a representação, mas não resolveu a aprendizagem. Uma combinação corretamente serializada não é necessariamente uma relação corretamente interpretada.

## Interpretação: avaliação final

| Modelo escolhido antes do teste | F1 macro de resultado | Quatro campos corretos | Claims brutos com limiar 0,7 | Falsos sucessos nesses claims |
| --- | ---: | ---: | ---: | ---: |
| E07 principal | 0,286 | 14/96 | 1/21 corretos | 7 |
| E18 por ação/observação | 0,237 | 14/96 | 1/4 corretos | 3 |

Os denominadores representam claims selecionados, não apenas previsões positivas. No E07, 1/21 corresponde a 4,8% de precisão integral, com intervalo Wilson de aproximadamente 0,8–22,7%. Sem rejeição, houve duas inversões graves no modelo principal. Matrizes de confusão, métricas por classe, jogo e curvas risco/cobertura estão nos [resultados completos](../research/frx_ai/v2/snapshot/final-results-v2.json).

Após calibração em um jogo separado, ambos aceitaram **zero claims**, cobertura zero e precisão indefinida. O ECE do principal caiu de 0,303 para 0,072, mas isso não tornou a interpretação útil. **A meta de 90% com cobertura útil não foi atingida.** Os conjuntos conformais não são certificados para novos jogos: a calibração é pequena e a hipótese de intercambialidade é inadequada.

A baseline Rust preservada encontrou dois claims, um integralmente correto; reconhece apenas variáveis, presentes em 12 das 96 propostas, e abstém em outros procedimentos. Essa cobertura não demonstra confiabilidade geral.

Erros centrais: sugestão confundida com tentativa executada; resultado de outra ação atribuído ao alvo; relato Windows transportado para contexto Linux; outro jogo mencionado dentro da thread; bundle reduzido a uma operação; retratações e alterações de versão. O caso real `descriptor_heap` permanece como diagnóstico conhecido de desenvolvimento: aparecem candidatos de regressão, mas também direção incorreta. Ele não demonstra generalização independente.

## Recuperação: avanço delimitado

Treinamos um dual encoder com objetivo contrastivo simétrico e múltiplos positivos, usando 117 comentários de treino; seleção em 39 comentários de desenvolvimento. O teste final utiliza os 96 comentários novos e **39 consultas respondíveis**, entre 64 consultas de sintomas predefinidas. São consultas em inglês por templates e relevância temática anotada provisoriamente; não são consultas reais de usuários nem prova de que uma solução funciona. Consultas sem relevante não entram nas médias: detecção de ausência de resposta continua pendente.

| Recuperador no teste final | Precisão@3 | NDCG@5 | MRR |
| --- | ---: | ---: | ---: |
| Sobreposição lexical | 0,308 | 0,421 | 0,554 |
| TF-IDF | 0,333 | 0,509 | 0,640 |
| MiniLM original FP32 | 0,513 | 0,658 | 0,731 |
| MiniLM original INT8, controle posterior | 0,487 | 0,659 | 0,740 |
| FRX contrastivo INT8 selecionado | 0,504 | 0,720 | 0,823 |

O modelo selecionado colocou o primeiro relevante no topo em 28/39 consultas, contra 22/39 do original FP32. **Precisão@3 caiu frente ao original FP32**; o avanço está principalmente na ordem dos primeiros resultados.

Uma auditoria **posterior à avaliação**, sem treinamento ou seleção adicional, exportou o original com a mesma quantização do candidato. Bootstrap pareado por jogo, 4.000 reamostragens, estimou intervalos de 95% para os ganhos: P@3 −0,016 a +0,048; NDCG +0,039 a +0,097; MRR +0,029 a +0,150. O ganho de P@3 permanece incerto. Há somente oito grupos de jogos e rótulos provisórios; esses intervalos descritivos não garantem desempenho populacional. O [controle completo](../research/frx_ai/v2/snapshot/matched-quantization-control.json) preserva sua natureza posterior.

A auditoria antiga também foi reproduzida: lexical P@3 0,406 e MiniLM 0,449, em material já conhecido. Ela não foi reutilizada como teste independente.

## Protótipo, recursos e reprodução

O [laboratório v2](../research/frx_ai/v2/README.md) contém CLI JSONL funcional, treino, aquisição ativa, avaliação, exportação ONNX e snapshots pequenos. O caminho principal usa aproximadamente **22,1 MiB de pesos para relações por objetivo e 21,9 MiB para recuperação**; incluindo tokenizers, aproximadamente 44,9 MiB. O comparador principal FP32 adicional tem 87 MiB e fica separado.

Inferência local não importa PyTorch, não exige Ollama e não baixa modelos implicitamente. Em quatro threads, o modelo por objetivo teve mediana de 14,4 ms com ação fornecida e 26,5 ms com propostas automáticas, em amostra pequena repetida de desenvolvimento. Cinco processos novos usando os dois modelos tiveram mediana de 0,542 s e pico de aproximadamente 196 MiB de RSS. Essas medidas não incluem rede; não equivalem a benchmark de produção. Pesos MiniLM de origem Apache-2.0 foram fixados por revisão e SHA-256; redistribuição requer avisos e revisão das licenças dos dados.

Os testes do laboratório passaram: **13 contratos novos e oito anteriores**. Eles verificam isolamento, spans, rejeição de entradas inadequadas e ausência de autorização; não provam qualidade linguística. A reconstrução dos dados pelos hashes e anotações foi conferida. Comentários originais, datasets hidratados e pesos ficam fora do Git; fontes alteradas não podem reutilizar offsets silenciosamente.

Arquivos desta rodada: `research/frx_ai/v2/` (código, protocolo, literatura, documentação e snapshots), link no README do laboratório anterior, ignore local de artefatos e este relatório. Os arquivos rastreados preexistentes foram comparados com seus hashes iniciais e preservados. Não houve mudança no aplicativo, Steam, prefixos, jogos, índice Git, commits ou push.

O protótipo produz candidatos com fontes, hashes, trechos e incerteza; `accepted_automatically` e `execution_authorized` permanecem falsos. Atribuição informada pelo chamador não autentica uma fonte remota. Próxima limitação científica: revisar rótulos independentemente e aprender propostas de eventos e relações por objetivo, incluindo correferência, restrições de ambiente e tentativas sucessivas. Português livre, inferência de causas e compatibilidade aprendida por hardware ainda não foram validados. A recuperação oferece uma direção útil; a interpretação ainda exige pesquisa.
