# Literatura — FRX AI v9

Consulta em 31/10/2026, pelo pesquisador (Claude) durante o desenvolvimento. **Nada disto é capacidade do protótipo**, que em execução não
chama nenhum serviço de IA. "Lido" indica a página ou o resumo citado, não o artigo inteiro. Métricas de artigos são do domínio deles
e não se aplicam ao FRX. Nenhum sistema foi reproduzido.

| Referência | Leitura | Mecanismo | Requisitos | Limite para o FRX | O que foi usado aqui |
|---|---|---|---|---|---|
| Yao et al. (2023). [ReAct](https://arxiv.org/abs/2210.03629) | Resumo | Intercala raciocínio e ações com fontes externas. Relata menos alucinação em HotpotQA e Fever | Um LLM grande que gera os passos | O FRX não tem modelo gerativo local | Só a estrutura ação → observação → decisão, feita por **regras**. Não é uma reprodução |
| Trivedi et al. (2023). [IRCoT](https://aclanthology.org/2023.acl-long.557/) | Resumo | Intercala recuperação e cadeia de raciocínio. Relata até 21 pontos em recuperação, com GPT3 | LLM e dados multi-hop com resposta na coleção | O ganho vem do LLM ler e escrever cada passo. Aqui, uma regra escolhe o termo seguinte | Motivou medir o ganho de cada nova busca. **Nossas regras não reproduziram esse ganho** |
| GitHub. [REST: Search](https://docs.github.com/en/rest/search/search) | Lido | 10 buscas/min sem autenticação; `q` até 256 caracteres; `search_type` padrão é lexical | Sem token para lexical | Semântica e híbrida **exigem autenticação** | Busca lexical, com `repo:`, `is:issue` e `created:<` |
| GitHub. [Busca aprimorada em Issues](https://github.blog/changelog/2026-04-02-improved-search-for-github-issues-is-now-generally-available/) | Lido | Semântica e híbrida sobre títulos e corpos | Autenticação, segundo a página REST | Fora do modo sem credenciais | Não usada |
| GitHub. [Boas práticas](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api), [limites](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) | Lido | 60 requisições/h sem autenticação; respeitar `retry-after`; requisições em série | n/a | Limites secundários não aparecem em `/rate_limit` | Espaçamento, espera e cache. **Vimos isso na prática** |
| OWASP. [Prompt Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html) | Lido, com trecho final resumido | Instruções em conteúdo externo; filtros "não são uma barreira de segurança"; permissões fora do modelo | n/a | Sem LLM, o risco principal vira dados envenenando consultas e relatórios | Limites impostos no código da ferramenta. O detector de hostilidade é só um sinal, nunca a barreira |

Não reli a literatura de v2–v8. Ela está nas respectivas bibliografias e na seção IX da [RXAI](../../../docs/RXAI.md#referencias).

**Não foi possível avaliar um modelo gerativo local:** esta máquina não tem Ollama nem llama.cpp, e escolhi não baixar pesos nesta tarefa.
Isso é uma lacuna deste experimento, não uma conclusão sobre modelos locais.
