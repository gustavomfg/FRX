# FRX AI v4 — resultados de 2026-10-30

A hipótese em duas etapas reduziu falsas execuções, mas **não resolveu compreensão de procedimentos**. O resultado útil é um laboratório auditável de eventos, verificação conservadora e investigação local. O detector treinado continua experimental; não há aprovação para integrá-lo ao Fix Engine. A vantagem neural sobre um extrator lexical simples não ficou demonstrada de forma robusta.

## Método e dados

Seis experimentos treinados, oito épocas cada; 34 leituras de desenvolvimento incluindo controles de decodificação/baselines; dois controles de exportação. Esses controles não são contados como outros 30 treinamentos. Também foram feitos uma avaliação final reservada, controles diagnósticos e verificações de engenharia.

A auditoria estrutural percorreu 305 registros antigos: 47 sem gatilho reconhecido pela heurística, 16 com possíveis múltiplos eventos, dois com possível metadado e três com spans amplos. **São sinais de revisão, não contagens de rótulos errados.** Por exemplo, uma variável válida pode não conter verbo reconhecido. Nenhum rótulo anterior foi sobrescrito e discordância com o modelo não foi usada como verdade.

A nova unidade separa gatilho, alvo literal, estado, direção, observação, grupo proprietário do resultado e temporalidade. Há ambiguidades reais: biblioteca não identificada, mudanças conjuntas, causas desconhecidas pelo autor e instruções que nunca foram executadas. `performed` significa execução/tentativa afirmada pela fonte; não significa conclusão da operação, sucesso ou observação pelo FRX. A ontologia ainda mistura ações corretivas, diagnósticas e alguns procedimentos de reprodução. `negated` também reúne execução negada e indisponibilidade; essa distinção merece revisão.

Foram anotados 45 parágrafos de treino/58 eventos/sete jogos e 24 de desenvolvimento/33 eventos/oito jogos. O desenvolvimento usa material antigo já conhecido. Os 160 contrastes sintéticos são exclusivamente de treino. Todos os rótulos foram feitos pelo agente, sem revisão humana independente.

Antes de ler os textos finais, foram reservados cinco AppIDs distintos dos jogos de treino/desenvolvimento. O congelamento ocorreu em **2026-10-30 05:41:27 UTC**, antes da inspeção/anotação final. Os 57 parágrafos finais contêm 64 eventos, dos quais 33 são execuções. Jogos: Lies of P, Black Myth: Wukong, A Plague Tale: Requiem, Dead Space e Remnant II. Só houve um parágrafo elegível de Lies of P; os demais têm 14 cada. Seleção enriquecida em eventos, uma unidade por comentário, primeiras páginas públicas: não representa a prevalência geral. Dois parágrafos foram truncados pelo limite de tokens, sem exclusão do denominador.

O teste foi executado uma vez após a anotação. Modelos, critérios e arquivos congelados permaneceram inalterados. Ferramentas posteriores de reprodução, descrição de erros e isolamento de contexto estão identificadas como posteriores; não foram usadas para melhorar as métricas finais.

## Experimentos

MiniLM-L6-v2 recebeu cabeças por token para gatilho, estado e direção. Nos modelos ajustados, os pesos das duas últimas camadas também aprenderam. O verificador literal é determinístico; não é chamado de IA treinada. A perda combina CE de gatilho + 0,5 CE de estado + 0,25 CE de direção, com 0,05 SupCon nos experimentos contrastivos (temperatura 0,12). S1–S4 usam peso positivo oito; S5/S6 usam frequência de tokens limitada a 80. O recuperador v2 permaneceu separado.

| Experimento | Hipótese | F1 de gatilho no desenvolvimento | Execuções corretas/aceitas após verificador |
| --- | --- | ---: | ---: |
| S1: spans antigos | Alvos amplos poderiam supervisionar eventos | 0,060 | 1/1 |
| S2: gatilhos auditados | Melhor unidade de anotação seria suficiente | 0,211 | 0/0 |
| S3: auditados + contrastes sintéticos | Contrastes e ajuste do encoder ajudariam a localizar eventos | 0,478 | 7/7 |
| S4: encoder congelado | Bastaria aprender as cabeças | 0,053 | 0/0 |
| S5: peso por frequência de classe | Rebalancear melhoraria cobertura | 0,474 | 8/9 |
| S6: S5 sem perda contrastiva | Isolar contribuição dessa perda | 0,464 | 8/9 |

S3 foi escolhido no desenvolvimento, antes do teste final. S1 é um controle do desajuste entre unidade antiga e gatilho, não uma comparação de qualidade de produto em tarefas idênticas. S3 versus S2 altera dados e perda; não isola sozinho o efeito contrastivo. S5 versus S6 não demonstrou vantagem relevante dessa perda. O aumento de cobertura com rebalanceamento trouxe erro. Projeção de cabeças/limites de palavras foram controles de decodificação, sem novos pesos. Todas as comparações definitivas usam IoU de caracteres ≥0,5; os históricos de treino também guardam uma métrica exploratória de sobreposição frouxa, que não deve ser anunciada como qualidade final.

## Teste em jogos não vistos

| Pipeline | Execuções corretas/aceitas | Falsas execuções | Recuperação das 33 execuções |
| --- | ---: | ---: | ---: |
| Detector neural sem verificador | 14/41 | 27 | 42,4% |
| Detector + verificador selecionado | 5/5 | 0 | 15,2% |
| Proposta lexical + mesmo verificador | 7/8 | 1 | 21,2% |
| Controle com gatilhos corretos + cabeça v3 | 28/49 | 21 | 84,8% |
| Controle com gatilhos corretos + verificador | 8/8 | 0 | 24,2% |

Os dois últimos controles recebem os gatilhos anotados; **não medem extração ponta a ponta**. A redução de falsos positivos nesses controles é acompanhada por perda grande de cobertura. Bootstrap pareado por jogo: redução de falsas execuções por evento entre 0,231 e 0,425, com apenas cinco grupos desiguais; não é garantia populacional.

No pipeline selecionado, precisão/recall de gatilhos = 0,594; 36 limites exatos entre 38 correspondências. Só 8/16 afirmações de estado + direção estão integralmente corretas. Houve 18 erros de direção entre os 38 eventos encontrados. As cinco execuções aceitas concentram-se em dois jogos e incluem uma ação de relato diagnóstico. O intervalo Wilson descritivo para 5/5 é aproximadamente **56,6%–100%**; dependência entre comentários torna a interpretação ainda mais limitada. Não foram atingidos os pisos do protocolo: 30 afirmações e quatro jogos com aceitação. Não há evidência para anunciar precisão geral ≥90%.

Todas as citações/identidades emitidas passam pela integridade literal de offsets; zero violações observadas, cinco identidades literais. Isso não garante interpretação correta. O esquema registra bibliotecas desconhecidas como desconhecidas e não transforma um nome de DLL em tecnologia comprovada.

**Resultados e procedimentos conjuntos não avançaram o suficiente:** zero associações recuperadas entre 20 observações de referência, mesmo no controle fornecendo eventos/estados corretos. O parser local de resultados ficou estreito demais. Zero falsos sucessos aqui não é êxito: nenhuma afirmação de sucesso foi produzida. Relações ficam desabilitadas por padrão. Temporalidade só possui suporte restrito ao marcador explícito `then`; não há generalização demonstrada de correferência.

## Investigação e memória

O diagnóstico reutiliza v3 e acrescenta sinais literais de função não implementada, driver Vulkan rejeitado e extensão ausente. Uma exceção com DLL/função leva à pergunta sobre origem, implementação e versão do componente; não a uma DLL inventada para instalar. Sintoma sem log conserva causa desconhecida e solicita evidência pertinente. Não foi treinado um modelo probabilístico de causa raiz, nem atribuído posterior numérico sem dados justificáveis. Comentários da mesma thread são agrupados, sem contá-los como confirmações independentes.

Em 12 controles históricos selecionados, incluindo três visões com telemetria deliberadamente omitida, o próximo tipo de verificação coincidiu com a referência provisória em 12 casos no v4 e 11 no v3. Só um exemplo difere. Há viés de seleção pelos parsers; **não é benchmark independente de diagnóstico**, nem demonstra ganho estatisticamente estabelecido de seleção por informação. Coletar pares reais de incidente → próxima pergunta continua pendente.

A memória v3 foi encapsulada por namespace com AppID, GPU, driver, Proton, versão do jogo, tecnologia, configuração e sintoma. Conserva tentativas do chamador e marca candidatas já falhas no mesmo contexto, sem transformar sucesso em outro ambiente em recomendação. Contratos isolados verificam persistência após reinício e divergências. O teste de integração nativa também executou seis cenários com processos separados e contexto explicitamente sintético: reteve a tentativa no reinício, recusou repetição sem justificativa e não reutilizou histórico ao mudar AppID, GPU, driver ou build. Após o teste de linguagem, uma revisão encontrou que o namespace protegia memória persistida, mas não tentativas fornecidas diretamente com outra versão do jogo. `run.py` adiciona validação dessa entrada e três contratos. É correção de engenharia posterior, sem alteração dos modelos/conclusões de linguagem congelados.

## Demonstrações com fontes reais

Os comandos do README geram seis investigações com trechos, URLs, hashes e offsets. São exemplos conhecidos, sem valor de novo teste independente:

| Caso | Trecho literal da fonte | Comportamento e limite |
| --- | --- | --- |
| [Sugestão](https://github.com/ValveSoftware/Proton/issues/6510#issuecomment-1422761622) | “You could try launching” | `launching` sugerida; não executada. |
| [Tentativa sem sucesso](https://github.com/ValveSoftware/Proton/issues/175#issuecomment-418249643) | “I replaced”; “same system hang as before” | Substituição reconhecida como relatada; resultado permanece sem associação automática. |
| [Regressão descriptor_heap](https://github.com/ValveSoftware/Proton/issues/9595#issuecomment-6001922940) | “caused badly distorted cloth/geometry and a different crash” | Falha exposta: detector encontra a remoção, mas perde a tentativa responsável pelo relato. Não declara sucesso. |
| [Procedimento conjunto](https://github.com/ValveSoftware/Proton/issues/607#issuecomment-436641081) | “I uninstalled”; “installed”; “Everything is running smoothly now” | Reconhece duas ações; não atribui sucesso individual. Associação automática permanece desativada. |
| [Dependência/função](https://github.com/ValveSoftware/Proton/issues/5415#issuecomment-996232308) | “unimplemented function POWRPROF.dll.PowerRegisterSuspendResumeNotification” | Solicita verificar componente/função/versão. Não inventa pacote instalável ou causa completa. |
| Causa desconhecida | Visão controlada de sintoma de um relato real | Sem log autorizado nessa entrada, solicita log e não confirma causa. URL/hash original no manifesto de demos. |

Os contratos verificam sugestões, negação, estado anterior e grupos de ações. Uma auditoria contrafactual posterior, com oito gatilhos fornecidos, acertou oito estados, mas só cinco direções: formas como `enabling`/`disabling` não são corretamente normalizadas pelos prefixos do verificador. É uma falha concreta registrada, sem alterar o sistema congelado depois de olhar o teste. Esses casos sintéticos não equivalem a generalização independente em linguagem real.

## Recuperação, custo e reprodução

O replay do conjunto v3 agora conhecido reproduziu exatamente o recuperador preservado: FRX v2 NDCG@5 **0,794**, MiniLM original INT8 **0,725**, BM25 **0,568**. FRX v2: MRR 0,945; Precision@3 0,634; Recall@5 0,682. Nenhum ganho novo de recuperação é reivindicado.

Na CPU Ryzen 5 4600G, quatro threads, os seis treinos somaram **802 s**. Não houve novo download de modelo. Cache novo ficou em aproximadamente 635 MiB, dentro do orçamento de 2 GiB. Pico de RAM de treino não foi instrumentado. O orçamento de seis GiB de RAM e 60 minutos de trabalho de modelos não deve ser confundido com uma cota imposta por cgroup.

Inferência: detector ONNX float 90,4 MB + recuperador v2 INT8 22,95 MB, total **113,35 MB** de pesos. INT8 do detector foi comparado antes do teste, mas perdeu uma das sete execuções corretas no desenvolvimento; float foi preservado. Sem PyTorch carregado: inicialização dos modelos 0,348 s; mediana 40,7 ms; p95 79,1 ms; VmHWM 242,8 MiB. São 24 repetições de seis exemplos pequenos conhecidos, sem rede; não é latência de produção nem desempenho de jogo. Paridade de conteúdo PyTorch/ONNX: 24/24 unidades de desenvolvimento.

Há **58 contratos de pesquisa aprovados**: 21 v4 e 37 preservados de v1/v2/v3. Reconstrução dos 126 parágrafos anotados a partir do cache reproduziu exatamente os hashes dos três arquivos. O selo original continua válido. As alterações anteriores do repositório foram preservadas; verificação de hashes e estado está no snapshot de preservação. Nenhum teste/build do aplicativo foi necessário, pois seu código não mudou nesta execução.

Arquivos novos estão exclusivamente em `research/frx_ai/v4/`: protocolo/anotação, detector/treino/decodificação, verificador, investigação/memória, CLI, avaliação/exportação, reprodução e snapshots. Corpos públicos, logs originais, datasets reconstruídos e pesos ficam fora do Git. Não houve alterações na Steam, jogos, prefixos, configurações, empacotamento, commits, tags ou push.

O próximo obstáculo é **aprender argumentos e vincular observações a eventos/grupos com cobertura útil**, separando também ações corretivas de relato/reprodução. Precisa de revisão humana dos spans e rótulos, mais exemplos reais de vários jogos e um novo teste independente. A rodada demonstrou o custo da rejeição conservadora; não demonstrou ainda um cérebro autônomo confiável.
