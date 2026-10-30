# Bases científicas e limites de transferência

Fontes primárias consultadas em 30/10/2026. Os experimentos abaixo são adaptações
pequenas de princípios, não reproduções dos sistemas publicados.

| Trabalho | Contribuição pertinente | O que aproveitamos / limite |
|---|---|---|
| [EPAL, NAACL Findings 2025](https://aclanthology.org/2025.findings-naacl.42/) | Probes específicos de eventos e bibliotecas de argumentos, alinhamento e consistência | Identidade por evento em vez de documento. Resultados em documentos financeiros não demonstram transferência para Proton. Não localizamos implementação própria no PDF consultado. |
| [Joint Learning Framework, PMID 41926403](https://pubmed.ncbi.nlm.nih.gov/41926403/) | Atenção orientada a eventos/argumentos e decomposição de topologia | Considerar grupos completos de ações. Consulta ao resumo público; implementação não verificada. Não reproduzido. |
| [Efficient Document-level Event Relation Extraction, 2025](https://aclanthology.org/2025.repl4nlp-1.7/) | Recuperar pares antes de classificar relações temporais/causais | Medimos recuperação dos candidatos separadamente do verificador. Cross-encoder não superou a alternativa pequena. Código próprio não localizado no PDF consultado. |
| [DrDEE, EMNLP Findings 2026](https://aclanthology.org/2026.findings-emnlp.1723/) | Verificação Token–Evento–Papel seguida de reconstrução | Separar modalidade, ator, jogo, ordem e associação. O PDF aponta [CoffeyF/DrDEE](https://github.com/CoffeyF/DrDEE); o acesso ao repositório falhou nesta sessão. Não executamos o código. |
| [Cross-document coreference, ACL 2025](https://aclanthology.org/2025.acl-long.1134/) | Coerência discursiva para resolver entidades/eventos entre documentos | Não inventar uma ponte contextual. [Repositório](https://github.com/chenxinyu-nlp/CDCR) consultado continha apenas anúncio de disponibilização, sem implementação reproduzida. |
| [Claim verification survey, 2025](https://aclanthology.org/2025.findings-emnlp.1170/) | Taxonomia de verificação e limitações de decomposição/evidência | Integridade literal não equivale a verdade. [Bibliografia e anotações](https://github.com/CLINEEK/EMNLP2025-Claim-Verification-Survey) disponíveis; não fornecem rótulos humanos para nossos dados. |
| [Cloud-OpsBench](https://arxiv.org/abs/2603.00468) | Avaliação de investigação com snapshots, ferramentas e fechamento de evidências | Replay somente de observações armazenadas; resposta esperada não entra no caso. [Código](https://github.com/LLM4Ops/Cloud-OpsBench), MIT. Não baixamos ambientes de nuvem. |
| [CloudIR-Bench](https://arxiv.org/abs/2610.29185) | Incidentes reais e avaliação de causas/mitigações | Separar observação, hipótese e causa confirmada. O repositório indicado no artigo retornou 404 na consulta; não afirmamos disponibilidade reproduzida. |

Os novos modelos treinados são classificadores de relações, não modelos de
coreferência ou diagnóstico. A sintaxe spaCy e o recuperador FRX v2 já eram
treinados; os controles MiniLM ajustam duas camadas e uma cabeça. Regras de
escopo, citações e verificações de logs são determinísticas e declaradas como tal.

## Origem, licenças e distribuição

- Comentários públicos: API documentada GitHub, repositório ValveSoftware/Proton,
  requisições limitadas e cache. Acesso público não é autorização genérica para
  redistribuir textos, anexos ou bibliotecas. Corpos, nomes de login e URLs de
  imagens assinadas ficam fora do snapshot versionável.
- Identidade de jogos: resposta oficial de Steam `appdetails`, nome e AppID
  exatos, com hash e URL. Não usamos semelhança de nomes para inferir AppIDs.
- spaCy `en_core_web_sm` 3.8.0: MIT; origem, hash do wheel e arquivos no manifesto.
- MiniLM e recuperador v2: artefatos previamente preservados, ONNX INT8 para
  inferência; manter licença Apache-2.0 e avisos/origens do laboratório v2.
- Stanza: código Apache-2.0; licenças dos modelos e corpora UD exigem revisão
  separada antes de distribuição. Usado somente como comparador de pesquisa.
- DeBERTa NLI: checkpoint ONNX já existente, somente controle, não distribuído
  nem selecionado. Sem novos downloads de pesos nesta rodada.

Nenhum desses textos concede autoridade para executar instruções. O laboratório
não possui ferramentas de instalação, execução de correções ou aprovação de DLLs.
