# FRX AI v9 — relatório

Data: 31/10/2026. Regras fixadas antes: [PROTOCOL.md](PROTOCOL.md). Métricas sem texto de comentários: [`snapshot/`](snapshot/).

**Rótulos e julgamentos são H0, do Claude. Não são validação humana.** Nenhum modelo foi treinado. Em execução, o protótipo não chama nenhum
serviço de IA, e planejamento e reformulação são regras.

## Conclusão

**A H9 não se sustentou.** Com orçamento igual, a investigação iterativa trouxe o conceito ao relatório em 1 de 17 casos, igual às duas pesquisas
únicas (diferença 0,0; IC [0,0; 0,0]). O critério exigia vencer B, B2 e F com IC acima de 0, e isso não ocorreu.

**Descoberta principal:** o gargalo não está em pesquisar, e sim em **entender o que foi lido.** Com a própria resposta como consulta, um teto
impossível na prática, o conceito foi lido em 16 de 17 casos e chegou ao relatório em apenas 1.

O que foi demonstrado de fato: consultas reais, trilha rastreável, segurança da ferramenta e casos isolados em que uma pista levou a outra.
Isso é infraestrutura e automação, **não inteligência investigativa**.

## Experimento congelado

**Dados:** snapshot real de `ValveSoftware/Proton`, cortado na data de cada caso. **Casos:** 24 novos, sorteados por estrato depois do congelamento;
17 com explicação documentada no próprio thread, usados na métrica primária. Orçamento: até 3 buscas e 6 leituras.

| Condição | Conceito no relatório | Conceito lido | Top-3 ok, de 24 | Buscas | Leituras | Relevantes (título) | Citações literais |
|---|---:|---:|---:|---:|---:|---:|---:|
| A. Sem internet | 0/17 | n/a | 19 | 0 | 0 | n/a | n/a |
| B. Pesquisa única | 1/17 | 3/17 | 17 | 0,96 | 2,71 | 20/62 | 50/50 |
| B2. Única, técnica | 1/17 | 3/17 | 17 | 0,96 | 4,04 | 21/95 | 63/63 |
| F. Três consultas fixas | 0/17 | 2/17 | 19 | 2,92 | 5,75 | 28/137 | 16/16 |
| R. Pista sorteada | 0/17 | 2/17 | 17 | 2,21 | 5,50 | 28/128 | 53/53 |
| **C. Iterativa** | **1/17** | **4/17** | 17 | 2,21 | 5,50 | 26/130 | 57/57 |
| S. C + MiniLM congelado | 1/17 | 3/17 | 16 | 2,46 | 5,50 | 30/132 | 71/71 |

| Comparação pareada | Conceito no relatório, IC 95% | Conceito lido, IC 95% |
|---|---|---|
| C − B | 0,0 [0,0; 0,0] | +5,9 pontos [0,0; 17,6] |
| C − B2 | 0,0 [0,0; 0,0] | +5,9 pontos [−11,8; 23,5] |
| C − F | +5,9 pontos [0,0; 17,6] | +11,8 pontos [0,0; 29,4] |
| C − R | +5,9 pontos [0,0; 17,6] | +11,8 pontos [0,0; 29,4] |
| C − A | +5,9 pontos [0,0; 17,6] | n/a |
| S − C | 0,0 [−17,6; 17,6] | −5,9 pontos [−23,5; 11,8] |

**Leitura:**

- **Internet contra catálogo local:** não houve ganho claro. Na próxima verificação, o catálogo local até empata com o melhor resultado (19/24), e C
  fica em 17/24. Pesquisar não melhorou a recomendação.
- **Seguir pistas contra pesquisa única:** sem vantagem demonstrada. Um único caso a mais em "conceito lido" não sustenta conclusão.
- **MiniLM congelado:** não ajudou. A busca semântica não foi o que faltou.
- **Só cerca de 1 em cada 5 documentos** parece relevante, e só 1 a 3 são aplicáveis. A pesquisa única teve a melhor taxa (32%), com menos documentos.
  **Limite:** o julgamento foi cego à condição, mas só por título.

### Ganho entre pesquisas (C)

| Tipo de consulta posterior | Consultas | Sem thread novo | Com pista nova | Com conceito novo |
|---|---:|---:|---:|---:|
| Especializada (usa termo lido) | 9 | 2 | 2 | 0 |
| Contextual (pista + GPU) | 4 | 0 | 3 | 1 |
| Verificação (pista com relatos opostos) | 1 | 0 | 1 | 0 |
| Ampliação (sem pistas, sem o jogo) | 7 | 0 | 1 | 0 |
| Consulta reduzida (zero resultados) | 9 | 9 | 0 | 0 |
| **Total** | **30** | **11** | **7** | **1** |

Das 30 consultas posteriores, 14 usaram algo lido, e **uma** trouxe um conceito novo. Nenhuma consulta de contradição chegou a ser feita dentro do orçamento.

### Onde cada resposta se perdeu

| Etapa em C | Casos |
|---|---:|
| O conceito existe no corpus antes da data, mas não foi lido | 13 |
| Foi lido, mas não chegou ao relatório | 3 |
| Chegou ao relatório | 1 |

Existir não significa que era achável: `dxvk`, por exemplo, aparece em 306 threads. Para distinguir as causas, rodei replays **depois da leitura**,
em dados conhecidos. **São diagnóstico, não teste.**

| Replay | Muda | No relatório | Lido | Insuficiente |
|---|---|---:|---:|---:|
| P1 | Uma busca com todo o texto do problema | 0/17 | 4/17 | 8 |
| P2 | **Oráculo:** a própria resposta como consulta | **1/17** | **16/17** | 12 |
| P3 | Iterativa, com termos raros do corpus como pistas | 0/17 | 5/17 | 3 |
| P4 | P1 com termos raros | 0/17 | 6/17 | 3 |

**P2 é o resultado que orienta o próximo passo.** Mesmo lendo o documento certo, o leitor quase nunca leva a resposta ao relatório. Causas que encontrei:

- **Vocabulário fixo:** só termos da lista escrita à mão viram pista, e `dmusic` ou `engine.ini` não estão nela.
- **Filtro de aplicabilidade:** exige termo em comum com a entrada, o que descarta trechos úteis e deixa 12 casos como "insuficiente" em P2.
- **Unidade errada:** o relatório pontua termos, não afirmações sobre o problema.
- **Termos raros não salvam:** eles aumentam o que é lido, mas nada chega ao relatório. A saída não é uma lista maior.

## Qualidade das conclusões

**Nenhuma resposta afirmou causa confirmada.** Mas pistas podem enganar. Revisão H0 dos 9 relatórios provisórios de C:

| Avaliação | Casos | Exemplo |
|---|---:|---|
| Correta | 0 | |
| Parcialmente pertinente | 4 | 9258 (anti-cheat), 8730 (rota de áudio), 9052 (`nvapi`), 8168 (`protontricks`) |
| Enganosa | 5 | 7390 apontou anti-cheat; a causa era `VK_DRIVER_FILES`. 8702 tomou `"resolved"` de um dump JSON como relato de resultado |

**Contradições:** C marcou 4 casos, mas a contradição é por termo. Um mesmo termo com relatos opostos em contextos diferentes conta como contradição.
Não verifiquei se são conflitos reais, então isso **não está demonstrado**.

**Abstinência:** em casos sem explicação documentada, C declarou insuficiência em 6 de 7. Em casos documentados, em 9 de 17, o que é honesto,
pois de fato não achou. Em um dos 7, relatou pistas sem pertinência (7737).

**Proveniência:** todas as 57 citações de C são literais, com hash correto, verificado por código.

## Ao vivo (complementar)

Sete casos, três condições, **21 investigações válidas** na API pública, sem credenciais, com 83 requisições e nenhuma rejeição pela lista permitida.
Não é reproduzível.

| Casos | Condição | Conceito no relatório | Conceito lido |
|---|---|---:|---:|
| 5 documentados de teste | B | 0/5 | 1/5 |
| | F | 0/5 | 2/5 |
| | **C** | **1/5** | 2/5 |
| Batman (**demonstração conhecida, não é teste**) | B / F / C | Não / Não / **Sim** | Sim / Sim / Sim |

- **Melhor exemplo real, em caso conhecido:** em Batman, `batman arkham city physx` → `physxdevice.dll performance` → `nvcuda.dll nvidia`. A terceira busca nasceu da
  evidência da anterior, e `nvcuda.dll` chegou ao relatório só em C.
- **Teste 7581 ao vivo:** C relatou `radv` e `amdgpu-pro`, mas por ampliação, sem seguir pista. Foi dizer "sem o jogo", não raciocinar a partir de evidência.
- **n = 5 não permite conclusão.** Além disso, `created:<` só corta a criação do thread, e o ranking ao vivo pode conhecer comentários posteriores.
- Os custos ao vivo dependem do cache compartilhado: C rodou depois de B e F, então suas leituras saíram baratas.

**Incidente:** na primeira rodada, o GitHub devolveu 403 de limite secundário com `/rate_limit` mostrando cota livre, e meu executor continuou tentando, em
13 execuções. Isso foi um erro meu, que a documentação alerta poder causar bloqueio. As execuções foram descartadas e registradas, e o código passou a espaçar
buscas entre processos e a esperar. Ninguém foi bloqueado depois disso. A lição científica: o limite real é mais baixo que o visível.

## Continuidade entre turnos

Quatro sequências preparadas sobre o corpus real. Sem respostas fixas: cada critério é uma propriedade do estado. **Testa atualização de estado, não diagnóstico.**

| Propriedade | Resultado |
|---|---:|
| Pede o log no turno 1 | 3/4 |
| Abandona o pedido de log depois de recebê-lo | 4/4 |
| Pesquisa o erro concreto recebido | 3/4 |
| Muda a próxima verificação no turno 2 | 4/4 |
| Enfraquece "biblioteca ausente" quando a pessoa diz que ela existe | 2/2 |
| Muda a conclusão no turno 3 | 1/2 |
| Registra a verificação que falhou | 4/4 |
| Não pede de novo a verificação refutada | 4/4 |
| Muda a decisão no turno 4 | 3/4 |
| Nunca confirma causa | 4/4 |

**Este é o resultado mais positivo do v9**, mas fixtures e catálogo têm o mesmo autor.

## Segurança

31 testes passam, sem tocar na rede:

| Risco | Verificado |
|---|---|
| URLs maliciosas | HTTP, outro host, `user@host`, porta, `file:`, IP local, metadata de nuvem, rota e repositório fora da lista |
| Destinos privados | Resposta de DNS privada bloqueia antes da requisição |
| Redirecionamentos | Para IP local, host estranho, HTTP e laços |
| Falhas | 403, 429 com `retry-after`, 404, resposta incompleta, excesso de tamanho, DNS, timeout, orçamento, cache |
| Credenciais | Nenhum cabeçalho `Authorization`; nada lê tokens |
| Conteúdo hostil | Instruções e `curl \| sudo bash` não viram pista, consulta, comando nem recomendação |
| Consultas | Termos de documentos não carregam `repo:`, `user:`, aspas nem operadores |
| Referências | Links para repositórios fora da lista não são seguidos |
| Mantenedor falso | Natureza vem dos metadados da API, nunca do texto |
| Saída | Sequências de terminal removidas; instruções de terceiros marcadas como citação |
| Evidência | Outro fabricante, outro jogo, fonte antiga, ciclo de consultas, falta de evidência |
| Escopo | Sem treino, sem shell, sem SDK de IA, só GET |

**Limite:** como não há LLM, injeção não pode "convencer" nada. Um modelo futuro precisaria de novos testes. O detector de hostilidade é só um sinal de
regex e pode ser contornado, mas os limites do código não dependem dele.

## Custo

| Medida | Valor |
|---|---|
| Decisão por caso, sem rede | 0,19 s em C; 0,42 s em S |
| CLI sem rede | 1,9 s e 542 MB, para carregar o índice de 76 MB |
| Ao vivo, sem cache | 3 a 16 s por investigação, sobretudo pelo espaçamento |
| Requisições, sem cache | Até 9: 3 buscas e 6 leituras |

## Desvios do protocolo

- **B2** foi acrescentada antes do teste, como controle extra.
- **`evaluate.py` foi corrigido** depois do sorteio e da trava do gabarito, antes de qualquer previsão: `import freeze` carregava o arquivo do v7. Só o avaliador mudou.
  Os dois selos estão preservados.
- **`net.py` mudou depois da leitura congelada**, só para o espaçamento ao vivo. Não afeta a busca congelada.
- **Coorte ao vivo:** 5 documentados + 1 não documentado + Batman, sorteados.
- **Modelo gerativo local:** não testado, porque a máquina não tem runtime e não baixei pesos.
- **Fontes além do GitHub:** não conectadas. Na prática, tudo foi para `ValveSoftware/Proton`, pois nenhuma consulta avaliada foi roteada a outro repositório.

## Limites

Autor único de regras, gabarito e julgamentos. Conceitos por texto literal, que perdem sinônimos e aceitam coincidências como `wayland`. Corpo congelado só
com um repositório. Julgamento só por título. Quase só inglês. Em threads de jogo novo, o próprio thread fica excluído, o que torna a tarefa difícil, e isso também
é realista. Nada foi verificado em um jogo real.

## Próximo passo

1. **Estudar o leitor de evidências, não mais conectores.** Medir sozinho: dado o documento certo, extrair afirmação, contexto e aplicabilidade.
2. Usar o P2 como banco de provas: 16 de 17 documentos já têm a resposta. É exatamente onde um modelo local congelado poderia ser comparado com as regras.
3. Depois medir de novo a pesquisa iterativa. Sem um leitor melhor, a comparação fica sem poder.
