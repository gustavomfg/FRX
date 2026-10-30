# Fundamentação e procedência

Consultado em 30/10/2026. As referências orientam hipóteses; não transferimos resultados de outros domínios para jogos Linux.

| Fonte primária | Contribuição verificada e uso nesta rodada |
|---|---|
| [DrDEE, Fang et al., EMNLP Findings 2026](https://aclanthology.org/2026.findings-emnlp.1723/) · [artigo](https://aclanthology.org/2026.findings-emnlp.1723.pdf) | Decompõe registros em verificação de triplas token–evento–papel e reconstrói eventos. Avaliação em eventos financeiros, não Proton. Inspirou nossa decomposição; não reproduzimos o modelo nem reivindicamos seus resultados. O link de implementação indicado no artigo não pôde ser inspecionado nesta sessão. |
| [SuperDRI, Microsoft Research](https://www.microsoft.com/en-us/research/publication/superdri-graph-guided-autonomous-troubleshooting-for-cloud-databases/) | Investigação orientada por grafo de incidentes históricos. Aproveitamos contexto, observações, verificações e procedimentos tipados. Não executamos agentes ou consultas de nuvem; a página pública consultada não forneceu implementação reproduzida nesta pesquisa. |
| [Cloud-OpsBench, repositório oficial](https://github.com/LLM4Ops/Cloud-OpsBench) | Avaliação de incidentes com estado, evidências e trajetória reproduzíveis. Adaptamos snapshots, cache de verificações e marcos separados dos dados de entrada. Não baixamos nem executamos o ambiente de nuvem. Repositório MIT; nenhum código externo foi copiado. |
| [Stanza: modelos](https://stanfordnlp.github.io/stanza/available_models.html), [modelo oficial inglês](https://huggingface.co/stanfordnlp/stanza-en) | Sintaxe neural pré-treinada como evidência estrutural. Stanza1.14.0, revisão de modelo `88e0cd3ddaa0c5a4b682895527ac013ce0241c21`, seis arquivos, hashes no snapshot. Não houve fine-tuning FRX desse parser. |
| [spaCy: modelo inglês](https://spacy.io/models/en), [release oficial3.8.0](https://github.com/explosion/spacy-models/releases/tag/en_core_web_sm-3.8.0) | Controle de custo e tamanho com outro parser treinado. Wheel MIT, SHA256 publicado, adaptação local das dependências para o esquema comum. Não é um modelo FRX treinado nesta rodada. |

## Licenças, integridade e confiança

O código Stanza e o model card consultado indicam Apache2.0. O card também informa termos adicionais para entradas do dicionário derivadas do SPECIALIST Lexicon; ver [atribuição oficial](https://stanfordnlp.github.io/stanza/combined_models.html#specialist-lexicon). Antes de distribuir pesos, revisar a revisão exata, avisos e componentes de dados usados. O controle spaCy indica MIT. A família MiniLM preservada tem procedência e licença registradas no laboratório v2.

Integridade é checada por SHA256 publicado e, para Stanza, também pelo MD5 do recurso. Isso verifica identidade dos artefatos, não demonstra qualidade nem ausência universal de risco. Modelos vêm apenas dos distribuidores oficiais identificados. Fontes, checkpoints e dependências ficam no cache de pesquisa; não alteramos instaladores nem baixamos modelos durante inferência.

Discussões públicas não equivalem a autorização irrestrita para redistribuir seus corpos ou dados pessoais. Por isso, o Git recebe somente nossas anotações, offsets, hashes e URLs, não os textos completos, logs ou checkpoints. Publicador e datas da fonte são preservados quando disponíveis. Nenhum comando presente em comentários é executado ou convertido em instrução com autoridade.

Os rótulos permanecem provisórios. Verificação literal, sintaxe treinada e escores semânticos são evidências auxiliares; nenhum deles autentica causalidade, segurança de uma correção ou resultado no computador do usuário.
