# Auditoria de proveniência — FRX AI v1–v7

Data: 30/10/2026. Escopo pedido: v2–v6. Incluí v1 e v7 porque os pesos e rótulos dependem deles.
Nada foi treinado, descartado ou alterado nos laboratórios. Este relatório não é análise jurídica e não
declara nenhum modelo proibido ou inválido.

Matriz completa, com hashes: [`provenance-matrix.json`](provenance-matrix.json). Gerada por
[`provenance_audit.py`](provenance_audit.py), somente leitura. Plano seguinte:
[`INDEPENDENT-EVALUATION-PLAN.md`](INDEPENDENT-EVALUATION-PLAN.md).

## Atualização de 30/10/2026: decisões do responsável

O texto abaixo está preservado como foi publicado, com as erratas. Decisões tomadas depois de ler a auditoria:

- **Proveniência:** a auditoria corrige a conclusão anterior. Os dados de treino de v1–v6 vieram do Codex, não do Claude.
- **Modelos:** pesos de v1–v6 ficam preservados em pesquisa. Sem distribuição e sem novos treinos com rótulos de proveniência
  contratualmente incerta, por enquanto. Avaliação interna e auditoria podem continuar.
- **Classificação:** se a FRX AI é ou não concorrente fica **sem determinação jurídica**. Não se afirma violação, nem que os pesos
  estão liberados para distribuição. O responsável vai pedir esclarecimento à OpenAI.
- **Termos:** registrados na seção XIII da [RXAI](../../../docs/RXAI.md#proveniencia), com a origem de cada item. As páginas continuam
  sem leitura direta. **As exceções do contrato empresarial não são aplicadas a uma conta ChatGPT Plus.** A atualização das Usage
  Policies só entra em vigor em 20/11/2026.
- **Benchmark e logs:** [piloto de 40 casos](../benchmark/PILOT-PROTOCOL.md), sem anotadores confirmados. Logs só de fixtures e fontes
  públicas, sem arquivos privados.

O item 3 da seção 8 continua aberto. O item 1 agora depende da confirmação indicada na RXAI.

## Conclusões

1. **Nenhum rótulo de v1–v6 foi gerado pelo Claude.** Os 13 scripts de anotação salvos e a lista inline do
   v3 foram escritos na sessão do **Codex (OpenAI), modelo `gpt-6.1-sol`**. A sessão do Claude não tocou
   esses caches antes de 30/10 22:40Z, quando os leu para o v7.
2. **Os 50 pesos treinados e salvos dependem de rótulos ou programas do Codex.** O único peso sem rótulos
   FRX é o controle MiniLM (`runtime-frozen-control`). Isso inclui os 2 componentes que o v7 reutiliza:
   L1 e, por `frx_lab.py`, o recuperador R02.
3. **O Claude produziu rótulos em outro lugar:** P7, J7 e C7 no v7, só para avaliação, e os antecedentes de
   29/10 (185 frases e os testes A/B do extrator). **Nenhum peso salvo depende deles.**
4. **O Claude escreveu as regras de D7** (532 demonstrações). `selector.py` treinou nelas durante o v7, mas
   não salvou pesos e nada disso é padrão.
5. **Nenhum rótulo teve revisão humana independente.** As filas de revisão (v2: 15, v4: 57, v6: 52 itens)
   não têm nenhuma resposta. Na sessão do Codex, desde 29/10 20Z, as 12 mensagens do usuário são pedidos de
   pesquisa e de Git.
6. **O aplicativo não contém modelo treinado da pesquisa.** Contém o extrator de regras `reports.rs`, escrito
   pelo Claude e medido com rótulos do Claude (seção 6).

## Errata das minhas afirmações anteriores

No v7, eu escrevi afirmações erradas ou mal fundamentadas. Os documentos do v7 e o RXAI agora apontam para
esta correção:

| Afirmação anterior | Onde | Correção |
|---|---|---|
| Os rótulos de v2–v6 são "do agente", lido como eu, e a questão vem dos Termos Comerciais da Anthropic | D-018; `v7/PROTOCOL.md`; `v7/REPORT.md`; `v7/LITERATURE.md`; RXAI IV-v7 | O agente de v1–v6 foi o Codex. Os termos da Anthropic não são o texto de partida para esses rótulos |
| Os Termos Comerciais (D.4) regem este uso | Os mesmos | A conta do Claude Code é Pro. A documentação do Claude Code associa Pro aos Termos do Consumidor e Team, Enterprise e API aos Comerciais |
| "Saída deste agente não é dado de treino" no v7 | `v7/PROTOCOL.md` | Vale para rótulos digitados. As regras de D7 são saída minha e geraram rótulos de treino de `selector.py`. Registrei isso como zona cinzenta, mas a frase geral é ampla demais |

## Método e limites

- **Autoria:** comparei as linhas mais longas de cada script com as chamadas de ferramenta das duas sessões,
  e conferi com os horários dos arquivos. Os 16 scripts batem com uma única origem (`annotator_authorship`).
- **Dependência:** `train_hash` de `v2/snapshot/experiments.json` confere com o arquivo esperado em 15 de 15
  execuções. Nas demais versões, a dependência vem da leitura do código de treino e dos manifestos.
- **Reuso:** `dev-known` do v5 tem o mesmo hash de `final-audited` do v4. `train-known` tem 69 linhas, igual a
  treino + desenvolvimento do v4.
- **Sem chamadas a outro modelo:** a busca por `claude`, `anthropic` e APIs nos comandos do Codex teve uma
  ocorrência, que é texto de documentação.
- **Não verificável daqui:** revisão humana feita fora das sessões e sem registro; o conteúdo interno de
  raciocínio do Codex; a fonte dos dados de treino dos modelos base (fora do escopo).
- **Hora em `first_at`:** é a primeira chamada que contém uma linha do script. Em scripts com trechos comuns,
  é uma aproximação; o autor não muda.

Identificadores de sessão, caminhos locais e informações de conta foram omitidos na versão pública. O script de auditoria recebe os caminhos por `FRX_CODEX_SESSION` e `FRX_CLAUDE_SESSION`.

## 1. Modalidade de acesso

Documentada sem abrir `~/.codex/auth.json` nem `~/.claude/.credentials.json`.

| Ferramenta | Evidência | Registro |
|---|---|---|
| **Codex** | `session_meta` e eventos de limite da sessão | Dados de conta e ambiente omitidos por privacidade; autoria técnica preservada |
| **Claude Code** | Campos não secretos de `~/.claude.json`; transcrição | Dados de conta e cobrança omitidos por privacidade; modelos `claude-opus-5-5` e `claude-haiku-5-5` |
| **Variáveis de ambiente** | Estado em 30/10 | Detalhes locais omitidos por privacidade |

As variáveis mostram só o estado atual, não o de cada sessão passada.

## 2. Datasets

Classes: **direto** = o modelo digitou o rótulo; **programa** = regra ou gerador escrito pelo modelo;
**misto** = os dois. Uso: *treino* e *ajuste* (seleção ou calibração) podem mudar pesos; *avaliação* só mede.

Todas as linhas têm **revisão humana independente: não**.

| Lab | Dataset | n | Classe | Autor | Uso |
|---|---|---:|---|---|---|
| v1 | train / dev / final | 100 / 50 / 80 | Direto | Codex | Treino / ajuste / avaliação |
| v2 | train-round1 → round2 → train-events | 87 → 113 → 167 | Direto | Codex | Treino |
| v2 | dev-events / calibration-events | 42 / 18 | Direto | Codex | Ajuste |
| v2 | final-events | 96 | Direto | Codex | Avaliação; depois ajuste do v3 |
| v2 | weak-events | 53 | Programa (`weak.py`) | Codex | Treino, só E14 |
| v2 | train / dev / calibration-goals | 183 / 44 / 20 | Misto | Codex | Treino / ajuste |
| v2 | final-goal-events | 96 | Direto | Codex | Avaliação |
| v2 | retrieval-train / dev | 144 / 39 | Direto (tópicos) | Codex | Treino / ajuste |
| v3 | final-annotations | 95 | Direto | Codex | Avaliação |
| v4 | train-audited / dev-audited | 45 / 24 | Direto | Codex | Treino / ajuste |
| v4 | final-audited | 57 | Direto | Codex | Avaliação; depois ajuste do v5 |
| v5 | train-known (reuso do v4) | 69 | Direto | Codex | Treino |
| v5 | dev-known (reuso do v4) | 57 | Direto | Codex | Ajuste |
| v5 | extra-audited | 14 | Direto | Codex | Treino |
| v5 | final-audited | 72 | Direto | Codex | Avaliação |
| v6 | training-pairs | 277 | Misto, ver abaixo | Codex | Treino |
| v6 | development-gold-v2 / acceptance-gold | 48 / 96 | Direto | Codex | Ajuste / avaliação |
| v7 | P7 / J7 / C7 | 77 / 40 / 24 | Direto | Claude Opus 5.5 | Avaliação |
| v7 | D7 | 532 | Programa (`demos.py`) sobre fala humana | Claude Opus 5.5 | Treino só em `selector.py` |

Composição de `training-pairs`:

| Texto | Grupo anotado (direto) | Contra-afirmações construídas por programa | Total |
|---|---:|---:|---:|
| Real | 45 | 88 | 133 |
| Sintético | 48 | 96 | 144 |

**Gerados no treino, sem arquivo rotulado (programa, Codex):** `synthetic.py` em v1, v2 e v4,
`training_data.py` e `relation_data.py` no v5, a simulação de `v3/structure.py` e `learning.py` no v6.

**O que deriva de fontes verificáveis:** o texto, a URL, o ID e a data de cada comentário público de
`ValveSoftware/Proton`, além do hash do corpo. **Nenhum rótulo categórico deriva diretamente da fonte:** estado,
resultado, relação, tópico e próxima verificação são interpretação de um agente. Em D7, a fala é de
mantenedores reais, mas a categoria vem de regex.

## 3. Pesos e suas dependências

Todos partem de modelos base públicos quando indicado. `Base` não entra na questão dos rótulos da FRX.

| Lab | Pesos | Treino | Ajuste | Base / pai |
|---|---|---|---|---|
| v1 | research-only, nli-head, reportability | train | dev | MiniLM, DeBERTa |
| v1 | augmented, focused | train + sintético | dev | MiniLM |
| v2 | E01, E02 | round1 | dev-events | MiniLM |
| v2 | E03, E04 | round1 + sintético | dev-events | MiniLM |
| v2 | E05, E06, **E07** | round2 + sintético | dev-events | MiniLM |
| v2 | E11–E13, E15, E17 | train-events + sintético | dev-events | MiniLM |
| v2 | E14 | train-events + sintético + **weak** | dev-events | MiniLM |
| v2 | E16 | train-events | dev-events | MiniLM |
| v2 | **E18** | train-goals + sintético | dev-goals | MiniLM |
| v2 | R01, **R02** | retrieval-train | retrieval-dev | MiniLM |
| v2 | runtime-relation / goal / retrieval | Herdado | + calibração | E07 / E18 / R02 |
| v2 | runtime-frozen-control | **Nenhum** | Nenhum | MiniLM |
| v3 | modality | v2 train + dev | v2 final | **R02** |
| v3 | outcome | v2 train + dev | v2 final | Nenhum |
| v3 | structure | Simulação | Nenhum | Nenhum |
| v4 | S1, S2 | v4 train | v4 dev | MiniLM |
| v4 | **S3**, S4–S6, runtime-events | v4 train + sintético | v4 dev | MiniLM |
| v5 | A1 | train-known | dev-known | MiniLM |
| v5 | A2–A4 | train-known + sintético | dev-known | MiniLM |
| v5 | A5, A6 | train-known + extra + sintético | dev-known | MiniLM |
| v5 | Linker (coeficientes em JSON; reprovado) | train-known + extra | dev-known | Nenhum |
| v6 | **L1**, N1 | Pares de texto real | development | Nenhum / MiniLM |
| v6 | L2, N2, N3 | Todos os pares | development | Nenhum / MiniLM |
| v6 | L3 | Todos os pares | development | **R02** |
| v7 | Cópia de L1 | Sem refit | Sem refit | L1 |

Negrito marca os pesos selecionados ou reutilizados. Também não foram salvos E08–E10, os seletores do v7 e os
classificadores da rodada 2. Parsers (spaCy, Stanza) e DeBERTa são pré-treinados externos, usados sem refit.

**Dependências que atravessam versões:**

- v3-modality e L3 dependem de R02, logo dos tópicos do `annotate_topics.py`.
- L1 depende de rótulos do v4 e do v5, reutilizados como treino.
- `v7/frx_lab.py` carrega R02 quando há busca de fontes, e `v7/final.py` usa L1 como comparador.
- `frx_lab.py --policy hybrid` e `case-prior` montam um índice de vizinhos sobre D7 em tempo de execução. O
  padrão `rules` não faz isso.

## 4. Resposta às perguntas do pedido

| Pergunta | Resposta |
|---|---|
| Rótulos gerados diretamente por Claude | P7, J7, C7. Antecedentes: 185 frases (Haiku 5.5) e testes A/B (Opus 5.5). **Nenhum em v2–v6** |
| Rótulos de regras ou programas escritos por Claude | D7. **Nenhum em v2–v6** |
| Rótulos diretos de outro agente | Todos os diretos de v1–v6: Codex |
| Rótulos de programas de outro agente | `weak-events`, pares de metas, sintéticos e contra-afirmações: Codex |
| Derivados de fontes verificáveis | Metadados e texto, não categorias |
| Com revisão humana independente | Nenhum |
| Pesos por dataset | Seção 3 e `weights` na matriz |
| Modalidade de acesso | Seção 1: ChatGPT Plus no Codex, Claude Pro no Claude Code |

**Código escrito por um agente não é rótulo.** Nos laboratórios, treinar e avaliar usam código do agente, o que
não torna rótulos os números que esse código mede. Separei os casos: o rótulo só é "programa" quando o programa
define a categoria, como em `weak.py`, nos sintéticos e em `demos.py`.

**"Manual" e "Human" nos registros antigos não significam pessoa.** `annotate.py` traz o comentário
*"Human source-based annotation by the research agent"*, o README do v1 fala em "100 manually labeled" e
`docs/FRX-AI-AVALIACAO.md` em "rotuladas manualmente". Nos três casos, quem rotulou foi um modelo. Não alterei
esses textos históricos; a RXAI registra a leitura correta.

## 5. Termos: pendência, sem conclusão

**Não há conclusão jurídica aqui.** O que precisa de esclarecimento é como cada texto se aplica aos dados acima.

### OpenAI: rege os rótulos de v1–v6

| Fonte oficial | Estado |
|---|---|
| [Terms of Use](https://openai.com/policies/row-terms-of-use/) | **Não lida:** HTTP 403 na leitura automática |
| [Usage Policies](https://openai.com/policies/usage-policies/) | **Não lida:** HTTP 403 |
| [Services Agreement](https://openai.com/policies/services-agreement/) | **Não lida:** HTTP 403 |
| [Uso do Codex com plano ChatGPT](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan) | **Não lida:** HTTP 403 |

Um buscador resumiu que os termos ao consumidor listam o uso de Output para desenvolver modelos concorrentes
entre os usos vedados, e que o Services Agreement tem exceção para classificadores não distribuídos. **É resumo
de terceiro, sem o texto primário nem vigência: não serve de base.** Este é o maior buraco da auditoria: a
maior parte dos pesos depende dos termos que não consegui ler. O responsável precisa ler essas páginas no
navegador e, se quiser, passar o texto para registro.

### Anthropic: rege P7, J7, C7, D7 e os antecedentes

Lidas em 30/10/2026:

| Fonte oficial | Vigência | Trecho |
|---|---|---|
| [Can I use my Outputs to train an AI model?](https://support.claude.com/en/articles/12326764-can-i-use-my-outputs-to-train-an-ai-model) | 16/03/2026 | "We prohibit customers from using our services to train or develop AI models without our written permission."; "You can use Claude's Outputs to train models that don't compete with Anthropic's own models." Exemplos permitidos: análise de sentimento, categorização, extração de informação, busca semântica, detecção de anomalias. Exemplos vedados: chatbots gerais, geração aberta de texto |
| [Usage Policy](https://www.anthropic.com/legal/aup) | Página mostra 12/11/2026; **versão vigente hoje não identificada** | "Use outputs to train an AI model (e.g., "model scraping" or "model distillation") without prior authorization from Anthropic" |
| [Consumer Terms](https://www.anthropic.com/legal/consumer-terms) | 08/10/2025 | Vedado "To develop any products or services that compete with our Services", "including to develop or train any artificial intelligence or machine learning algorithms or models" |
| [Commercial Terms](https://www.anthropic.com/legal/commercial-terms) | 17/06/2025 | D.4; associados a Team, Enterprise e API |
| [Claude Code: legal e conformidade](https://code.claude.com/docs/en/legal-and-compliance) | Sem data lida | Consumer Terms para Free, Pro, Max e Ultra; Commercial Terms para Team, Enterprise e API; Usage Policy para todos |

**Tensões de texto a esclarecer:**

- As condições diferem: "written permission", "prior authorization", "expressly approved".
- A qualificação "concorrente" aparece no artigo de suporte e nos termos, mas não na frase citada da Usage Policy.
- O artigo permite treinar modelos não concorrentes e também lista "Using Outputs as training targets" como vedado.
- O [artigo sobre Covered Models](https://support.claude.com/en/articles/15425695-covered-models) trata de retenção e
  não fala de treino com saídas. Opus 5.5 e Haiku 5.5 não aparecem na lista lida.

**Perguntas abertas, para os termos de cada provedor:** a FRX AI, especializada em jogos Linux e compacta,
é "concorrente"? O uso conta como "classificador" ou "extração"? Importa o modelo ser distribuído? Rótulos
gerados por regra escrita pelo modelo contam como saída para treino? A rota mais direta é pedir esclarecimento
por escrito a cada provedor.

## 6. O aplicativo distribuído

Verificado por `checks.app`: nenhum arquivo de pesos no Git e nenhuma referência a ONNX, joblib, laboratórios ou
modelos fora de `research/` e `docs/`.

**O que existe no aplicativo:** `src-tauri/src/fixes/reports.rs`, usado por `providers/github.rs`, classifica
resultados relatados por opção, com **regras fixas**, sem pesos aprendidos. O Claude Opus 5.5 escreveu essas
regras em 29/10 e as mediu com rótulos que ele mesmo fez (teste A, 80 pares; teste B, 40), ajustando regras
depois de inspecionar erros do teste A, como registra `docs/FRX-AI-AVALIACAO.md`. **Não é modelo treinado e fica
fora do congelamento de pesos**, mas seu resultado (9/9 no teste B) não tem validação independente. Ele entra
como comparador no plano. `cargo test` só o avalia com `FRX_AI_GOLD`, em teste ignorado por padrão.

## 7. Congelamento em vigor

Todo o histórico v1–v7 fica preservado. Até concluir a auditoria:

- **Sem treino novo com estes rótulos.** Isso inclui rodar de novo `train.py`, `retrieval_train.py`,
  `train_atomic.py`, `train_linker.py`, `learning.py`, `neural.py` e `v7/selector.py`, que treina em D7.
- **Reprodução de treino também fica suspensa,** mesmo com as instruções atuais nos READMEs.
- **Uso permitido dos pesos:** inferência congelada em pesquisa interna, como comparador.
- **Sem integração** de qualquer peso ao aplicativo.

Isso é regra de processo, documentada: nenhum script foi alterado para impor esse bloqueio, porque os laboratórios
ficam preservados como estão.

## 8. Para o responsável

1. Ler os termos da OpenAI (seção 5), que a automação não alcançou.
2. Decidir se pede esclarecimento por escrito à OpenAI e à Anthropic.
3. Definir quem serão os anotadores humanos do plano. Sem eles, o benchmark não pode ser chamado de independente.

## Reproduzir

```fish
python3 research/frx_ai/audit/provenance_audit.py
```

Exige os caches `~/.cache/frx-ai-rnd-*` e as duas sessões nomeadas no script. Regenera a matriz sem treinar.
