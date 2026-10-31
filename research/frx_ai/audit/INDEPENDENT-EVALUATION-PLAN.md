# Plano experimental — avaliação independente da FRX AI

Estado em 30/10/2026: **o [piloto de 40 casos](../benchmark/PILOT-PROTOCOL.md) está preparado, mas não anotado, e não há anotadores
confirmados.** O v8 mediu regras com rótulos H0, o que **não substitui nenhuma fase deste plano**. Os limites abaixo seguem valendo.
A Frente L usa por ora só fixtures e logs públicos.
Base: [auditoria de proveniência](PROVENANCE-AUDIT.md), RXAI D-013 (resultado não detectado responde por 41 de 70
perdas), D-016 e Q-005, Q-009, Q-010.

Meta inalterada: uma IA compacta, especializada em investigar e ajudar a resolver problemas de jogos Linux.

## Regras

1. **Este plano não treina nenhum classificador.** Só mede regras e pesos congelados.
2. **Rótulos de agente não são verdade de referência.** Nenhum modelo, Claude ou Codex, anota, sugere ou
   adjudica o benchmark. Agentes podem escrever ferramentas e calcular métricas.
3. **O gabarito fica selado para os agentes** até as previsões serem travadas por hash.
4. **Ambiente isolado:** sem alterar Steam, jogos, prefixos ou o aplicativo. Corpos de comentários e logs ficam
   fora do Git. Nada baixado é executado.
5. **Heurística é heurística.** A seção "Linguagem permitida" vale para todo relatório.

## Decisões do responsável

| ID | Decisão | Efeito se ficar em aberto |
|---|---|---|
| R1 | Quem são os 2 anotadores e o adjudicador | Fases 1–4 não começam com nível H2 |
| R2 | Quais logs reais podem ser usados | A Frente L fica só com logs públicos |
| R3 | Termos e pedido de esclarecimento aos provedores | Não bloqueia avaliação; bloqueia treino futuro |

**Níveis de revisão, sempre declarados por rótulo:**

| Nível | Definição | Pode ser chamado de independente? |
|---|---|---|
| H0 | Só agente | Não |
| H1 | Uma pessoa, inclusive o responsável | Não: "revisão de uma pessoa" |
| H2 | Duas pessoas às cegas, sem ver previsões nem sugestão de modelo, com adjudicação | **Sim** |

O responsável pode ser anotador, desde que às cegas, mas como desenvolve o sistema isso fica registrado. Um
resultado só é "independente" em H2.

## Fase 0 — Infraestrutura e protocolo

**Entregas:** guia de anotação; ferramenta local de anotação sem previsões; lista de exclusão de fontes;
registro de proveniência por rótulo; verificador do selo.

**Pool:** comentários e threads públicos, excluindo toda URL já usada em v1–v7. A amostra é sorteada por semente
fixa dentro de estratos pré-definidos, nunca escolhida por um agente. Separação por thread e AppID.

**Registro por rótulo:** `annotator_id`, nível, data, versão do guia, hash da fonte, declaração de "sem assistência
de LLM" por lote.

| Critério de saída | Como verificar |
|---|---|
| Nenhuma sobreposição de URL ou thread com v1–v7 | Script compara com snapshots; resultado 0 |
| Sorteio reproduzível | Mesma semente gera o mesmo hash de amostra |
| Protocolo e guia pré-registrados | Hash antes do primeiro rótulo do conjunto selado |
| Ferramenta não mostra previsão | Teste de contrato: a ferramenta não importa nem lê previsões |
| Ruído de template conhecido filtrado | Teste com os casos de D-017 |

## Fase 1 — Benchmark independente (IB-1)

**Unidade:** um comentário e, quando houver, a ação relatada. **Campos:** estado da ação, afirmação de resultado,
trecho literal de evidência, ator e jogo.

| Conjunto | Tamanho | Uso |
|---|---:|---|
| Piloto de concordância | 50 | Ajustar o guia; depois fica conhecido |
| Desenvolvimento | 100 | Único permitido para escrever regras |
| Selado | ≥ 400, até ≥ 100 afirmações positivas | Medição única |

**Estimativa de esforço (a confirmar no piloto):** cerca de 2 min por unidade, ou 37 h-pessoa para dois anotadores
nas 550 unidades, fora a adjudicação.

| Critério de saída | Limite |
|---|---|
| Krippendorff α em estado e resultado, no piloto | ≥ 0,67 para continuar; **≥ 0,80 no selado** para chamá-lo de confiável |
| Concordância de trecho de evidência | Sobreposição F1 ≥ 0,80 |
| Adjudicação | 100% das discordâncias, com motivo registrado |
| Revisão | 100% do selado em H2 |
| Selo | Hash do gabarito registrado antes de qualquer previsão |

Se α ficar abaixo de 0,67 em um campo, o campo é redefinido ou removido, e isso é um resultado, não uma falha a esconder.

## Fase 2 — Detector de resultados isolado (Q-009)

**Pergunta:** este texto afirma um resultado sobre o estado do jogo, qual e com qual evidência literal?

Medido **sozinho**, sem gerador de eventos nem associação ação–resultado.

**Comparadores, todos congelados:**

| Id | Sistema | Natureza |
|---|---|---|
| C0 | Sempre se abster; classe majoritária | Referência |
| C1 | `reports.rs` do aplicativo, medido sem alterá-lo | Regras |
| C2 | `v2/weak.py` | Regras |
| C3 | Regras de observação do v7 | Regras |
| C4 | E07 do v2, `outcome` do v3, L1 do v6, onde couber | Pesos congelados; só comparação |
| C5 | Nova regra escrita **só** com o desenvolvimento | Regras |

Se a tarefa não couber em um comparador, isso é registrado e ele fica fora da comparação.

**Métricas, com IC de 95% por thread:** precisão e cobertura de "resolveu"; contagem de falsos "resolveu"; F1 macro
com as 4 classes; precisão da evidência literal; taxa de abstenção; resultados por estrato (inglês, template, log
colado, negação, contra-afirmação).

| Critério pré-registrado | Limite |
|---|---|
| Precisão de "resolveu" | Limite inferior de Wilson ≥ 0,90, com ≥ 100 afirmações (na prática, ≥ 96 de 100) |
| Falso "resolveu" | Publicar a contagem e o limite superior. Zero erros em 59 dá ≤ 5%; em 100, ≤ 3% |
| Cobertura | Limite inferior ≥ 40% dos resultados reais |
| Evidência | 100% das saídas com trecho literal da fonte |
| Teste único | Selado rodado uma vez, com hash do código registrado antes |

**Teste de utilidade:** nas perdas de D-013, medir quantos resultados passam a ser encontrados, separado da decisão.
Aprovar o detector **não** o conecta ao aplicativo: isso exige decisão à parte.

## Fase 3 — Frente L: compreensão de logs reais

Paralela à Fase 2, depois da Fase 0.

**Dados:** logs públicos de issues do Proton, baixados só como texto, sem executar nada; logs do responsável, se R2
permitir. Dados pessoais são removidos antes de anotar e não entram no Git.

**Referência mais forte:** quando existir, a causa confirmada por **fonte externa verificável**, como a conclusão do
mantenedor ou o commit de correção, com URL. Fica separada do rótulo humano e marcada como E1. A interpretação do
anotador é E2.

**Tarefas e métricas:**

| Tarefa | Métrica |
|---|---|
| L1. Encontrar linhas de erro relevantes | Precisão e cobertura por linha, com relevância anotada em H2 |
| L2. Identificar o componente com falha | Exata ou equivalente, com abstenção |
| L3. Distinguir ruído comum de sinal (`fixme:` etc.) | Falsos alarmes por log |
| L4. Dizer o que o log **não** permite concluir | Taxa de causa inventada, meta **0** |

**Comparadores:** busca literal de padrões e `v7/signals.py`.

| Critério | Limite |
|---|---|
| Tamanho | ≥ 100 logs, ≥ 30 threads, ≥ 20 com referência E1 |
| Concordância em relevância | α ≥ 0,67 |
| Decisão | Só é "compreensão" o que vencer a busca literal com IC pareado acima de 0, e sem causas inventadas |

## Fase 4 — Frente T: trajetórias de diagnóstico (Q-010)

**Esquema, passo a passo, cada um com URL, ator, data e trecho:**

`problema → evidência → hipótese → verificação → resultado → decisão seguinte`

Hipóteses implícitas ficam marcadas como inferência do anotador. Trajetórias sem desfecho são preservadas como
abertas: não se completa o que a fonte não diz.

| Conjunto | Tamanho |
|---|---|
| Piloto | 20 threads |
| Selado | ≥ 80 threads, com ≥ 1 ciclo completo cada |

O tamanho vem de estimativa de planejamento: ≈ 71 threads para detectar 15 pontos de vantagem pareada, com poder de
80% e desvio assumido de 0,45. **O desvio é suposição, recalculada no piloto.** Vantagem de 10 pontos pediria ≈ 159.

**Replay incremental:** a cada passo o sistema vê só o passado. Prevê a próxima verificação ou se abstém.

**Métricas:**

- Próxima verificação: top-1 e top-3 contra o conjunto aceitável em H2, e contra o que o mantenedor pediu.
- Verificação que separou hipóteses, marcada por anotadores.
- Pedidos inúteis: informação que já estava no histórico.
- Atualização correta após resultado: mudou a decisão quando devia?
- Causa afirmada sem suporte: meta **0**.

**Comparadores:** sempre pedir o log; maioria; regras do v7; replays congelados. **Estado atual: nenhum modelo mostrou
vantagem consistente na seleção de verificações.** Este experimento serve para testar isso, sem pressupor resultado.

| Critério | Limite |
|---|---|
| Concordância em "aceitável" e "decisão seguinte" | α ≥ 0,67 no piloto; ≥ 0,80 no selado |
| Vantagem declarada | IC pareado de 95% por thread acima de 0, contra o melhor comparador |
| Replicação | Repetida em um segundo conjunto selado antes de virar conclusão |
| Causa sem suporte | 0 |

## Portão para qualquer treino futuro

Fora deste plano, e só depois de: auditoria concluída; decisão registrada sobre os termos; benchmark H2 existente;
detector medido; dados de treino com proveniência declarada linha a linha, separados do benchmark; e hipótese
pré-registrada dizendo qual comparador precisa vencer.

## Ordem

| Etapa | Depende de | Saída |
|---|---|---|
| Fase 0 | Nada | Ferramenta, guia, pool, verificador |
| Pilotos das Fases 1 e 4 | R1 | α, guia ajustado, esforço e desvio reais |
| Fase 1 selada | Piloto | IB-1 selado |
| Fase 2 | Fase 1 | Relatório do detector, leitura única |
| Fase 3 | Fase 0, R2 | Relatório de logs |
| Fase 4 selada | Piloto | Relatório de trajetórias |

**Regras de parada:** α abaixo de 0,67 depois de duas revisões do guia encerra o campo. Selado aberto antes da hora
vira desenvolvimento e exige novo sorteio. Qualquer desvio é publicado, mesmo que favoreça o sistema.

## Linguagem permitida

| Temos | Podemos dizer | Não podemos dizer |
|---|---|---|
| Regras e catálogo de verificações | "Regras sugerem esta verificação" | "Diagnóstico inteligente" |
| Recuperador R02 | "Busca por tópico, experimental" | "Encontra a correção" |
| Extrator de eventos | "Candidatos com evidência literal" | "Entende o procedimento" |
| Seletor de verificações | "Não superou o baseline" | "Aprende a investigar" |
| Métrica com H0 | "Medido contra rótulos provisórios de agente" | "Validado" |
| Métrica com H2 selado | "Avaliação independente em N casos" | "Funciona em jogos" sem teste em jogo real |
