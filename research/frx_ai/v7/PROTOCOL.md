# FRX AI v7 — protocolo pré-registrado

Registrado em 30/10/2026, depois da leitura de v1–v6 e do Experimento A sobre as
previsões já publicadas de v6, e **antes** de coletar ou abrir qualquer dado novo.
Somente `research/frx_ai/v7/`, `docs/RXAI.md` (entrada v7) e o cache externo
`~/.cache/frx-ai-rnd-v7-20261030` podem mudar. Sem commit, push, tag, alteração do
aplicativo, da Steam, de jogos, prefixos ou drivers. Nada recuperado é executado.

## Decisão de proveniência (seção 9 do pedido)

> **Errata de 30/10/2026:** esta seção está preservada, mas contém erros. Os rótulos de v1–v6 são do Codex,
> não deste agente; a conta é Pro, e a documentação associa esse plano aos Termos do Consumidor; e as regras de
> D7 são saída deste agente. Ver [auditoria](../audit/PROVENANCE-AUDIT.md) e RXAI E-001.

Os rótulos de avaliação continuam sendo produzidos por este agente, um modelo
comercial. Os Termos Comerciais da Anthropic (vigentes desde 17/06/2025, D.4)
vedam usar o serviço para construir produto concorrente, "including to train
competing AI models". Não cabe a este laboratório decidir se a FRX AI se enquadra.
Portanto, no v7:

- **Saída deste agente não é dado de treino.** Rótulos do agente só avaliam.
- O seletor aprendido da Frente B treina em **demonstrações humanas públicas**
  (pedidos de mantenedores em issues), rotuladas por funções determinísticas.
  Essas funções foram escritas pelo agente: isso é registrado como questão aberta.
- O verificador L1 de v6 foi treinado com rótulos provisórios do agente. Ele é
  reavaliado congelado, sem refit, e a questão fica registrada para o responsável.
- Nenhuma IA professora comercial é usada; pesos abertos não são baixados sem
  hipótese mensurável.

## Hipóteses

- **H1 (Exp. A):** as 59 perdas de v6 concentram-se antes do verificador, em
  resultado não detectado, filtro de modalidade e identidade do gatilho.
- **H2 (Exp. B):** com candidatos corretos, o verificador recupera bem mais que
  9/70, mas distante do total, porque recebeu treino em outro gerador.
- **H3 (Exp. C):** separar *proposta* de *decisão* — modalidade como atributo, não
  filtro; resultado por cláusula; ação por verbo técnico, gerúndio, `using/with` e
  conjunção — eleva a cobertura de candidatos completos em relação a v6 no mesmo
  teste novo, com custo e candidatos falsos medidos.
- **H4 (Exp. D):** representar o procedimento (`single`, `joint`, `sequence`,
  `alternatives`) recupera conjuntos completos sem conceder crédito individual.
- **H5 (Frente B):** seleção aprendida da próxima verificação supera ordem fixa,
  maioria e recuperação de casos, em discussões novas. **Pode ser refutada.**

## Dados

| ID | Uso | Estatuto |
|---|---|---|
| P6-dev (48) | desenvolvimento do gerador | conhecido |
| P6-M, P6-C (96) | Exp. A/B e desenvolvimento do v7 | **conhecido a partir de agora; deixa de ser teste** |
| P7-M | teste novo do gerador e das relações | reservado antes do congelamento |
| P7-J | desafio de procedimentos com várias ações | reservado; relatado à parte |
| D7 | demonstrações de mantenedores (Frente B) | divididas por discussão |
| C7 | casos de investigação completa | reservados antes das previsões |

Discussões de P7: issues ValveSoftware/Proton com AppID no título, fora de todos
os AppIDs usados em v1–v6, escolhidas por hash com semente fixa entre as com pelo
menos 8 comentários. Por discussão: até 6 unidades com ação e resultado, 2 com
múltiplas ações, 1 pedido e 1 resíduo por hash (P7-M); e até 3 unidades com três ou
mais ações (P7-J), sem repetir comentário. Um parágrafo, 40–1300 caracteres. A
amostra é enriquecida por construção e não estima prevalência.

Ordem obrigatória: reservar sem exibir corpos → congelar código do gerador por
hash → anotar P7 integralmente → travar rótulos por hash → produzir previsões →
uma avaliação final. Sem ajuste depois. Erros encontrados depois entram no relatório
e só seriam corrigidos para um teste futuro. Rótulos permanecem provisórios do
agente, sem revisão humana independente. Não reclassificar a favor do modelo.

## Métricas e comparações

Protocolo de P6 preservado: gatilho com IoU de caracteres ≥0,5, um para um; relação
correta exige resultado, interseção do trecho e conjunto completo de proprietários;
denominador são observações com proprietários e resultado conhecido. Adicionalmente,
variante leniente declarada: o gatilho também casa com o verbo leve anotado
(`tried` em `tried fiddling`). Ambas são publicadas para **todos** os comparadores.

- **Candidatos:** cobertura, precisão, candidatos por documento, tempo.
- **Relações:** corretas/propostas, recall, falsos sucessos, negativas corretas.
- **Conjuntos:** grupo completo recuperado; crédito individual indevido a partir de
  procedimento conjunto conta como erro crítico, separadamente.
- **Causalidade:** nenhuma saída pode afirmar `causality_confirmed`.
- Comparadores: v5 Stanza, v6 selecionado, v6 estrutural, v7, todos no mesmo P7.
- Bootstrap por discussão. Números de P6 e P7 nunca são somados.

Critério de sucesso fixado agora: cobertura de candidatos em P7 **pelo menos duas
vezes** a de v6 no mesmo conjunto, com até 8 candidatos por documento em mediana;
falsos sucessos de v7 não maiores que de v6 em taxa por relação proposta. Se falhar,
registrar como negativo.

## Frente B

Espaço fechado de verificações, definido antes de ver D7 em detalhe. Política
comparada: maioria, ordem fixa por regras, recuperação de casos (kNN), logística
sobre TF-IDF e estado, logística sobre embeddings do recuperador v2, e híbrida.
Divisão por discussão e sem AppID compartilhado; seleção por validação cruzada em
grupos no treino; o teste é lido uma vez, depois de congelar a política. Métrica
principal: a verificação escolhida está entre as pedidas pelo mantenedor (top-1),
com top-3, MRR e recall por classe. Limitação conhecida: o pedido do mantenedor é
demonstração de prática, não prova de utilidade diagnóstica.

C7: casos reais não usados no desenvolvimento, com critérios escritos antes da
execução: compreensão, evidência, hipóteses, próximo passo, segurança, eficiência.
Se os dados não bastarem, registrar o que falta. Saída de regra, recuperação, modelo
e usuário é sempre identificada. Hipótese nunca vira causa confirmada por regra.

## Recursos e parada

CPU, até 8 threads; RTX 4060 detectada, sem CUDA no venv: não usada. Sem novos pesos
salvo justificativa. Treino curto; sem busca de hiperparâmetros sem hipótese. Parar
ao obter um resultado medido ou um platô documentado.
