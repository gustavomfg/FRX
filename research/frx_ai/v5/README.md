# FRX AI v5 — Evidence-driven incident investigation

Laboratório local de pesquisa. Não altera o aplicativo, não executa correções e não transforma previsões em permissões do Fix Engine.

O avanço desta rodada é a decomposição de relatos em operações, modalidade, observações e grupos de ações. A extração usa sintaxe neural pré-treinada e reconstrução determinística. O recuperador treinado FRX v2 permanece intacto. Os novos verificadores treinados não atingiram qualidade suficiente para uso automático.

Na avaliação prospectiva, com 72 trechos de seis discussões/AppIDs novos, o v5 recuperou **28/52 ações executadas**, contra **14/52 no v4**. Fez **quatro falsas execuções** e **uma falsa atribuição de sucesso**. Recuperou **6/9 relações propostas**, incluindo duas pertencentes a outros jogos mencionados na mesma discussão. Portanto, a extração continua experimental. Todos os rótulos são provisórios, produzidos pelo agente, sem revisão humana independente.

O [relatório](REPORT.md) explica as limitações, os controles e os resultados por capacidade. [Fontes científicas e licenças](LITERATURE.md), [protocolo](PROTOCOL.md) e [anotação](ANNOTATION.md) registram a metodologia.

## Executar no ambiente desta pesquisa

Na raiz do repositório:

```bash
FRX_V5_CACHE="$HOME/.cache/frx-ai-rnd-v5-20261030"
FRX_V2_CACHE="$HOME/.cache/frx-ai-rnd-v2-20261029"
FRX_V4_CACHE="$HOME/.cache/frx-ai-rnd-v4-20261030"
FRX_V5_PY="$FRX_V5_CACHE/venv/bin/python"

"$FRX_V5_PY" research/frx_ai/v5/seal.py "$FRX_V5_CACHE" "$FRX_V2_CACHE" --verify
"$FRX_V5_PY" research/frx_ai/v5/make_examples.py "$FRX_V5_CACHE" "$FRX_V4_CACHE"
"$FRX_V5_PY" research/frx_ai/v5/run.py "$FRX_V5_CACHE" "$FRX_V2_CACHE" < "$FRX_V5_CACHE/examples.jsonl"
```

Entrada e saída usam uma linha JSON por caso. `run.py` é a entrada pública; acrescenta um bloqueio de atribuição por jogo após a auditoria final. `lab.py` conserva a inferência congelada usada na avaliação. Esse bloqueio posterior **não conta como melhora das métricas**: deixa `event_app_id=null`, preserva `source_thread_app_id` e impede apresentar identidade da discussão como identidade verificada de cada evento.

Os exemplos incluem sugestão não executada, tentativa malsucedida, regressão com `descriptor_heap`, procedimento conjunto, erro de função de biblioteca, causa desconhecida e incompatibilidade de hardware. Parte vem de relatos públicos históricos; contextos e logs fictícios são explicitamente identificados como controles. Nada representa telemetria nova do computador.

Para memória experimental, acrescente `--memory /caminho/externo/memoria-v5`. O caminho é escolhido na CLI, nunca pelo texto recuperado. Somente tentativas estruturadas e atribuídas pelo chamador entram no histórico; previsões comunitárias não viram tentativas locais.

Cada caso precisa de `context.app_id`, `symptom` e, opcionalmente, `log`, `evidence`, `attempts`, `candidate_actions` e `facts`. Evidências contêm `app_id`, `url`, `text` e contexto conhecido. Tentativas contêm identidade, ação estruturada, resultado e ambiente completo. Veja os exemplos materializados; dados ausentes permanecem desconhecidos. A saída inclui citações com offsets Unicode e hashes, eventos, relações experimentais, hipóteses, próximas verificações, rejeições de contexto e limitações.

## Reproduzir os resultados

Os checkpoints e corpos de comentários ficam fora do Git. `snapshot/` contém rótulos com offsets, URLs, hashes, métricas, seleção e registro dos experimentos. A reconstrução usa os corpos originais do cache; `--fetch` autoriza explicitamente a API pública GitHub, com limite por execução e falha se o conteúdo mudou.

```bash
FRX_V5_REPLAY="$HOME/.cache/frx-ai-v5-replay"
"$FRX_V5_PY" research/frx_ai/v5/hydrate.py "$FRX_V5_REPLAY" "$FRX_V5_CACHE" "$FRX_V2_CACHE" --body-cache "$FRX_V5_CACHE/bodies"
"$FRX_V5_PY" research/frx_ai/v5/final_evaluate.py "$FRX_V5_REPLAY" "$FRX_V4_CACHE"
```

Isso reproduz uma avaliação **já conhecida**, não cria um teste independente. Os scripts recusam sobrescrever resultados finais existentes. A reconstrução local foi testada sem rede; produziu as mesmas métricas dos cinco comparadores.

Para repetir ajustes, prepare outro diretório com `hydrate.py ... --for-training`. Ele carrega somente treino/desenvolvimento e os 14 casos adicionais. Execute `train_atomic.py DIRETORIO FRX_V2_CACHE CONFIG` e `train_linker.py DIRETORIO --name linker-audited`. `A6-seed-control` restabelece a semente após extrair atributos: a inicialização preguiçosa do parser consumia RNG e confundia os ajustes A1–A5 conforme o cache estivesse frio ou preenchido. O comportamento histórico desses controles está preservado; suas métricas não são comparações causais limpas entre arquiteturas. O diretório original está congelado e recusa novos ajustes. Réplicas históricas não permitem reutilizar o teste final para selecionar uma abordagem nova.

Se os modelos não estiverem em cache, crie um venv externo, instale `requirements-research.txt` e execute explicitamente `setup_syntax.py DIRETORIO_MODELOS` e `setup_compact.py DIRETORIO_MODELOS`. Os scripts verificam versões, tamanhos e hashes publicados. O recuperador v2 e o controle v4 são os artefatos preservados dos laboratórios anteriores. **A inferência não baixa modelos.** Nenhum modelo foi adicionado aos instaladores do FRX.

```bash
"$FRX_V5_PY" -m unittest discover -s research/frx_ai/v5 -p 'test_*.py'
```

São 14 testes de contratos e isolamento; não substituem a avaliação de linguagem. Para executar em cache alternativo, defina `FRX_V5_CACHE` no ambiente desse comando.

## Custos e limites

O protótipo selecionado tem aproximadamente 266 MB de pesos: 243 MB de sintaxe e 23 MB de recuperação. Em CPU, o processo completo atingiu 797 MiB de RSS, primeira requisição de 3,46 s, mediana aquecida sem cache de 142 ms e P95 de 348 ms, em 12 requisições. Ambiente Python e bibliotecas não estão incluídos no tamanho dos pesos. O controle spaCy é menor e mais rápido, mas precisa de uma nova avaliação antes de substituir a seleção congelada.

A próxima limitação é a atribuição **ator/jogo/ambiente → ação → resultado**, incluindo condições e estado preexistente. A memória evita repetir uma tentativa explicitamente registrada no mesmo contexto; não aprende causalidade nem descobre sozinha qual alternativa funciona. As verificações de logs reutilizam lógica determinística do v3/v4. Não há modelo de diagnóstico calibrado, instrumentação de jogo ou validação humana independente nesta entrega.
