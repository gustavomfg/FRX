# Eventos, contexto e revisão

Os rótulos são provisórios e foram produzidos pelo agente. Revisão automática,
evidência literal e concordância com um modelo não equivalem a validação humana.

Cada unidade conserva URL, discussão, documento/comentário, conta publicadora,
datas de publicação/edição distintas, hash do corpo completo, offset do parágrafo
e hash do trecho. Esses metadados não provam quem executou uma ação nem qual
jogo ela afetou. A identidade do evento exige suporte no próprio texto.

Eventos têm trigger, alvo literal opcional, direção, modalidade, evidência de
modalidade e identidades de jogo/ator. Modalidades: `performed`, `suggested`,
`negated`, `preexisting`, `unknown`. O ator pode ser `reporter`, `addressee`,
`quoted`, `third_party` ou `unknown`; uma citação não fornece automaticamente a
identidade da conta citada. Sujeito conhecido não prova execução concluída.

Resultados têm trecho, classe e conjunto completo de proprietários. Com várias
ações, o resultado conjunto não concede crédito causal a cada uma. Um resultado
parcial de inicialização pode coexistir com crash posterior ou erro ao salvar.
Configurações condicionais e preexistentes podem ter associações relatadas
anotadas, embora o gerador v6 ainda só proponha relações para ações executadas.
Essa limitação permanece no denominador e reduz a recuperação.

O teste foi anotado por inteiro antes de produzir previsões, após congelar
código/modelos/limiares. A primeira execução abortou por offsets invertidos em
discurso indireto; `offset-repair.json` registra a correção mínima, os hashes
antes/depois e a ausência de métricas inspecionadas. Não houve ajuste semântico
com base no teste. O congelamento original foi preservado.

## O que a métrica significa

- Correspondência de triggers: IoU de caracteres >=0,5, um para um.
- Uma previsão executada sem correspondência executada conta como falsa. Isso
  inclui erros reais, eventos duplicados, diferenças de cabeça verbal e omissões
  da anotação. **Não significa que todos sejam erros semânticos de modalidade.**
- Relação correta exige mesmo resultado, interseção literal e conjunto completo
  de proprietários, sem recompensar duplicatas.
- “Falso sucesso” significa atribuição positiva incorreta, inclusive crédito
  individual indevido; não exige que a fonte negue qualquer melhoria.
- AppID incorreto conhecido e atribuição sem sustentação são contados
  separadamente. Um evento extra não anotado também pode tornar a atribuição
  estritamente incorreta. Zero trocas de AppID num conjunto quase sem pares de
  jogos explícitos não comprova resolução de coreferência entre jogos.
- As arestas temporais não foram exaustivamente anotadas na aceitação. Não há
  estimativa confiável dessa competência.

Depois do teste, a auditoria encontrou divergências como `Tried fiddling` anotado
em `fiddling`, mas extraído em `Tried`, ou `try running` extraído em `running`.
Também há software `forces you` indevidamente tratado como ação humana e a
negação `no versions ... get it working` interpretada positivamente. **Os
rótulos e resultados finais não foram reescritos para favorecer o modelo.**

`snapshot/human-review-queue.json` lista 52 unidades que merecem revisão;
`snapshot/acceptance-gold.json` fornece rótulos e offsets. O cache externo contém
texto integral e análise em `posthoc-errors.json`. A próxima revisão deve
canonicalizar o evento/trigger e anotar todas as relações, sem adotar a previsão
como verdade. A correção de três atores no desenvolvimento foi versionada
separadamente; os rótulos originais permanecem preservados.
