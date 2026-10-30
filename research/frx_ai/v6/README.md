# FRX AI v6 — Grounded Diagnostic Intelligence

Laboratório executável de investigação local. **A extração continua experimental:
o teste novo não confirmou interpretação confiável de procedimentos.** Nenhuma
previsão aprova correções ou entra no Fix Engine.

O protótipo combina o recuperador FRX v2 preservado, sintaxe compacta, identidade
literal por evento, candidatos de grupos de ações, um verificador treinado de
2,1 KB e verificações explícitas de contexto/logs. Mantém autor da fonte separado
do ator relatado, jogo da discussão separado do jogo do evento, citações com
offsets Unicode e contexto desconhecido como `null`.

Nos **72 trechos principais de seis discussões novas**, recuperou 19/43 ações
executadas e 7/48 relações ação–resultado; o v5 recuperou 12/43 e 4/48. Isso veio
com quatro falsas execuções na métrica estrita e duas atribuições incorretas de
sucesso, contra zero em ambos os indicadores no v5. Os 24 casos adicionais de
contexto foram piores. Todos os rótulos são provisórios, produzidos pelo agente,
sem revisão humana independente. Veja [resultados e erros](REPORT.md),
[protocolo](PROTOCOL.md), [anotação](ANNOTATION.md) e [literatura](LITERATURE.md).

## Executar neste ambiente

Na raiz do repositório:

```bash
FRX_V6_CACHE="$HOME/.cache/frx-ai-rnd-v6-20261030"
FRX_V2_CACHE="$HOME/.cache/frx-ai-rnd-v2-20261029"
FRX_V6_PY="$FRX_V6_CACHE/venv/bin/python"

"$FRX_V6_PY" research/frx_ai/v6/seal.py "$FRX_V6_CACHE" "$FRX_V2_CACHE" --verify
"$FRX_V6_PY" research/frx_ai/v6/demonstrate.py "$FRX_V6_CACHE" "$FRX_V2_CACHE"
"$FRX_V6_PY" research/frx_ai/v6/investigate.py "$FRX_V6_CACHE" "$FRX_V2_CACHE" < "$FRX_V6_CACHE/examples.jsonl"
```

Entrada e saída são JSONL. Os 11 exemplos incluem sugestão, tentativa falha,
regressão histórica `descriptor_heap`, relato citado, procedimento conjunto,
dependência, causa desconhecida, memória e hardware incompatível. São relatos
históricos conhecidos ou controles sintéticos identificados; não são novas
medições do computador. O exemplo `known-failure-negation` expõe deliberadamente
um erro remanescente, sem fingir que o extrator acertou.

Um caso requer `context.app_id` explícito e `symptom`. Campos opcionais:
`log`, `evidence`, `attempts`, `candidate_actions`, `facts` e `checked_items`.
Cada evidência informa `text`, `url` e, quando conhecidos, `app_id`, `context`,
`document_id` e `reporter_id`. O AppID da investigação não preenche a identidade
dos acontecimentos encontrados nas fontes. Uma URL permitida não autentica seu
conteúdo. A saída mantém associação relatada, hipótese e observação distintas;
`automatic_approval` e `execution_authorized` permanecem falsos.

Para memória persistente, acrescente `--memory /caminho/externo/memoria-v6`.
Somente tentativas estruturadas do chamador entram nela. A chave inclui ambiente
completo fornecido e sintoma; trocar driver, jogo, GPU, versão ou hash de artefato
não herda automaticamente um resultado. Fontes incompatíveis podem continuar
ordenadas como analogias, sem transferir eficácia ao caso atual.

## Reproduzir a avaliação já conhecida

Corpos, caches e pesos ficam fora do Git. `snapshot/` contém offsets, rótulos,
URLs, hashes, resultados, seleção, comparadores e fila de revisão. O comando
abaixo reconstitui os textos originais do cache, sem rede:

```bash
FRX_V6_REPLAY="$HOME/.cache/frx-ai-v6-known-replay-new"
"$FRX_V6_PY" research/frx_ai/v6/rehydrate.py "$FRX_V6_REPLAY" "$FRX_V6_CACHE" "$FRX_V2_CACHE"
"$FRX_V6_PY" research/frx_ai/v6/final.py "$FRX_V6_REPLAY"
"$FRX_V6_PY" research/frx_ai/v6/posthoc.py "$FRX_V6_REPLAY"
```

Os seis comparadores foram reproduzidos em outro diretório com caches de sintaxe
vazios e **métricas idênticas**, inclusive por jogo. Isso é replay conhecido,
não outra validação independente. Os scripts recusam sobrescrever avaliação
final existente. `--fetch` em `rehydrate.py` autoriza consultas limitadas à API
pública GitHub via `gh`, somente se o texto mantiver o hash original. Uma edição
na fonte provoca recusa, não substituição silenciosa.

Para refits conhecidos, use outro diretório e `rehydrate.py ... --for-training`;
isso não carrega a aceitação. `learning.py DIRETORIO FRX_V2_CACHE` ajusta os três
controles lineares. Depois `neural.py DIRETORIO FRX_V2_CACHE N1-real-text`,
`N2-contrast` ou `N3-structured-contrast` repete os ajustes neurais. A localização
do cache histórico v5 é derivada do caminho v2 padrão desta sessão. Os arquivos
originais `train-known.jsonl` e `extra-audited.jsonl` são necessários, assim como
o checkpoint MiniLM preservado em `FRX_V2_CACHE/pretrained`. Não use o teste
publicado para selecionar um novo modelo e chamá-lo de teste independente.

## Verificações e recursos

```bash
"$FRX_V6_PY" -m unittest discover -s research/frx_ai/v6 -p 'test_*.py'
"$FRX_V6_PY" research/frx_ai/v6/benchmark.py "$FRX_V6_CACHE" "$FRX_V2_CACHE"
"$FRX_V6_PY" research/frx_ai/v6/benchmark.py "$FRX_V6_CACHE" "$FRX_V2_CACHE" --baseline
"$FRX_V6_PY" research/frx_ai/v6/probe_diagnosis.py "$FRX_V6_CACHE"
```

23 testes de contratos passaram; a CLI executou os 11 casos e validou 106
citações com offsets ajustados contra os textos de entrada. Isso não substitui
avaliação linguística. Para outro cache de testes, defina `FRX_V6_CACHE`.
O protótipo selecionado executou
com importação de Torch bloqueada. Usa cerca de **38,7 MB de artefatos ativos**,
incluindo recuperador, sintaxe e classificador, sem contar Python/dependências.
O benchmark registra tempos, memória, primeira requisição e casos individuais em
`efficiency.json`; o [relatório](REPORT.md) contextualiza a comparação com v5.

Nenhum peso novo foi baixado. Em outro computador, prepare explicitamente um
venv externo com `requirements-runtime.txt` e os artefatos verificados dos
laboratórios anteriores. `v5/setup_compact.py` e `v5/setup_syntax.py` são etapas
explícitas de preparação, não downloads na inferência. Só os comparadores e
treinamentos precisam de `requirements-research.txt`. Este é um laboratório
Python; não há integração Rust, modelo no instalador ou nova versão do FRX.
