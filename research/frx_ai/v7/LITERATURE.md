# Literatura — FRX AI v7

Consulta em 30/10/2026. "Lido" indica as seções de método relevantes, não o artigo inteiro nem só o
resumo. "Parcial" indica resumo e trechos. Nenhum sistema foi reproduzido integralmente. Métricas de artigos são do domínio
deles e não se aplicam ao FRX.

| Referência | Leitura | Hipótese que ataca | Requisitos | Limites para o FRX | Justificativa e resultado aqui |
|---|---|---|---|---|---|
| Li, Wang, Du (2025). [Efficient Document-level Event Relation Extraction](https://aclanthology.org/2025.repl4nlp-1.7/). RepL4NLP | Lido | Comparar todos os pares é quadrático e desbalanceado; um retriever filtra antes do cross-encoder | Dois encoders treinados | **Parte de menções de evento dadas**: não resolve eventos e resultados ausentes | Motivou separar proposta de decisão. Nosso Exp. A mostrou zero perda na etapa de pares, então um retriever de pares não foi implementado |
| Su et al. (2026). [Suggest-Verify-Revise: A Three-Stage Document-Level Event Causality Identification with Narrative Consistency](https://aclanthology.org/2026.acl-long.871.pdf). ACL | Parcial | Causalidade documental com sugestões de LLM e de hipergrafo, verificação por processo de Hawkes e revisão | LLM e modelos estruturais | Depende de LLM e foca causalidade em notícias; sua separação sugerir → verificar é a parte útil | Só o princípio de estágios; não implementado. Confirmou que causa entre eventos é tarefa própria, que o FRX não afirma |
| Dalvi Mishra, Tandon, Bosselut, Yih, Clark (2019). [Everything Happens for a Reason: Discovering the Purpose of Actions in Procedural Text](https://aclanthology.org/D19-1457/). EMNLP | Parcial | Dependências entre passos de um procedimento explicam ações | Modelo treinado, ProPara | Textos de processo científico, ordenados; relatos de usuário não têm essa ordem | Motivou representar relações entre ações. **Representação por estrutura não recuperou mais grupos** (Exp. D) |
| Rao, Daumé III (2018). [Learning to Ask Good Questions: Ranking Clarification Questions using Neural Expected Value of Perfect Information](https://aclanthology.org/P18-1255/). ACL | Lido | Bons pedidos são os que têm resposta útil; candidatos vêm de perguntas feitas em posts parecidos (Lucene, top 10) | 77 mil posts e rede neural | Ranqueia perguntas livres; o domínio é StackExchange. P@1 de 27,7% contra 23,1% do baseline: ganho modesto | **Implementado como recuperação de casos k=15.** Selecionado na validação cruzada e não superou a maioria. A ideia de utilidade esperada motiva a próxima hipótese |
| Hagen, Deckers, Wolter, Scells, Potthast (2025). [Investigating Counterclaims in Causality Extraction from Text](https://arxiv.org/abs/2510.08224). Preprint | Lido | Modelos treinados só com afirmações causais erram contra-afirmações mais de 10× mais | Anotação com kappa 0,74 | Corpus de notícias reescrito manualmente | Motivou anotar negativas e testar negação; nosso erro `no versions ... get it working` é desse tipo |
| Yu, Li, Wang (2019). [Detecting Causal Language Use in Science Findings](https://aclanthology.org/D19-1473/). EMNLP | Parcial | Separa correlacional, condicional e causal direto | BERT, 3 mil frases | Conclusões de PubMed; a linguagem difere de relatos | Motivou `causality_confirmed=false` e não tratar `fixed` como causa. Nenhum classificador treinado |
| Ratner et al. (2020). [Snorkel: rapid training data creation with weak supervision](https://doi.org/10.1007/s00778-019-00552-1). VLDB Journal | Consulta (já em v2) | Funções de rótulo geram treino e têm a confiabilidade estimada | Várias funções sobrepostas | Com dez funções pouco sobrepostas o modelo generativo não se justifica | **Funções de rótulo usadas em D7**, sem o modelo generativo. A auditoria achou erro grave de template, reforçando que precisam de verificação |
| Anthropic (vigência 17/06/2025). [Commercial Terms of Service](https://www.anthropic.com/legal/commercial-terms), D.4 | Lido | Restrição contratual para treinar modelos concorrentes com o serviço | n/a | Não é análise jurídica; o enquadramento cabe ao responsável | **Saída do agente deixou de ser dado de treino no v7.** Questão aberta sobre v2–v6. **Errata:** ver [auditoria](../audit/PROVENANCE-AUDIT.md); esta linha não se aplica aos rótulos de v1–v6, que são do Codex |

Sem acesso: Chaparro et al. (2017), *Detecting Missing Information in Bug Descriptions* (a ACM devolveu
HTTP 403). Não citado como fundamento, pois não foi lido; seria a próxima leitura para triagem de relatos.

Não encontrei trabalho específico sobre atribuição conjunta de múltiplas intervenções em relatos de
usuário, nem sobre próximo passo diagnóstico em triagem de bugs a partir de pedidos de mantenedores.
A busca foi limitada: isso não prova inexistência.

Referências anteriores (DocRED, PURE, EPAL, DrDEE, SuperDRI, Cloud-OpsBench e outras) estão nas
bibliografias de v2 a v6 e na seção IX do [RXAI](../../../docs/RXAI.md#referencias).
