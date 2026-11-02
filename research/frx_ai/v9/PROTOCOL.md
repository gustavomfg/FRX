# FRX AI v9 — protocolo fixado antes dos dados

Data: 31/10/2026. Hipótese **H9**: a investigação online iterativa, orientada pelo contexto técnico, produz diagnósticos mais úteis e
fundamentados que conhecimento local ou uma pesquisa única. A hipótese será testada, não presumida.

## Limites

- **Nenhum treino.** Nenhum dataset de treino é produzido. Rótulos deste laboratório são H0, do Claude, só para avaliação.
- **O Claude não é o motor do protótipo.** Em execução, o protótipo não chama nenhum serviço de IA. Nesta máquina não há runtime de
  modelo gerativo local, então **planejamento, roteamento e reformulação são regras**. Modelo aprendido medido: só o MiniLM público
  congelado, como reordenador, e como condição separada.
- **Só leitura e só público:** GET sem credenciais em `api.github.com`. Sem baixar anexos, sem executar nada, sem arquivos privados.
- Fora das coortes: threads rotulados em v1–v8 e do piloto, que não é lido nem pontuado.

## Condições comparadas

Mesmos casos, mesmos dados iniciais, mesmas fontes, mesmos leitores e avaliadores.

| Id | Condição | Buscas | Leituras | O que isola |
|---|---|---:|---:|---|
| A | Sem internet: catálogo local do v7/v8 | 0 | 0 | Valor de pesquisar |
| B | Pesquisa única, lendo os melhores resultados | 1 | 6 | Linha de base com **o mesmo número de documentos** |
| B2 | Pesquisa única técnica, sem o nome do jogo | 1 | 6 | Se B perde só por uma consulta inicial ruim. Acrescentada antes do teste |
| F | Três consultas fixas por modelo, sem usar o que foi lido | 3 | 6 | **Mais consultas sem seguir pistas** |
| R | Iterativa, mas a pista seguinte é sorteada | ≤ 3 | 6 | Valor de **escolher** a pista |
| C | Iterativa: segue a pista escolhida por regra | ≤ 3 | 6 | Sistema proposto |
| S | C, com reordenação pelo MiniLM congelado | ≤ 3 | 6 | Valor da recuperação semântica |

**A comparação principal é C contra B, B2 e F, com orçamento igual.** Ganho vindo só de mais documentos não conta, porque todas as
condições online leem no máximo 6.

## Duas avaliações

1. **Corpus congelado (reproduzível, principal):** snapshot real de `ValveSoftware/Proton` do v7, com 9900 issues e 30000 comentários.
   A busca imita a lexical do GitHub: todos os termos precisam aparecer no thread, com ranking BM25. Tudo é cortado na data do caso.
2. **Ao vivo (complementar):** API pública do GitHub, com `created:<data` nas buscas e corte local dos comentários. Limites sem
   autenticação: 10 buscas por minuto e 60 leituras por hora. A internet muda, então isto não é reproduzível, e o ranking do GitHub pode
   ter vantagem de conhecer comentários posteriores.

## Coortes

| Coorte | Tamanho | Uso |
|---|---:|---|
| Batman | 1 | **Demonstração conhecida. Nunca é teste.** |
| `dev` | 6 | Escrever regras; erros podem ser lidos |
| `test` | 24 | Leitura única, sorteada **depois** do congelamento por hash |
| `live` | 6 do `test` | Mesmos casos, ao vivo, depois da leitura congelada |

Estratos do sorteio, 3 casos cada: biblioteca ausente, gráfico, regressão, driver, áudio, crash, desempenho e sem filtro. O caso é o
primeiro post do thread ou, em threads antigos, um comentário de problema posterior. O sistema vê o problema até a data de corte. O
restante do thread fica oculto.

**Gabarito H0, escrito antes de qualquer previsão:**

- `concepts`: termos do que resolveu ou explicou o problema, lidos na parte oculta e ausentes da entrada.
- `checks`: verificações aceitáveis do catálogo.
- `documented`: se o próprio thread chegou a uma explicação ou solução.

## Métricas, fixadas agora

**Primária: conceito no relatório.** Em quantos casos com `documented=true` o relatório final traz um conceito-chave nas pistas ou
hipóteses. Comparação pareada por caso, com IC bootstrap de 95%.

| Métrica | Definição |
|---|---|
| Conceito lido | Um conceito aparece em algum documento lido, mesmo sem chegar ao relatório |
| Relevância | Documentos lidos sobre o mesmo problema; julgamento H0 cego à condição |
| Aplicabilidade | Documentos do mesmo jogo ou contexto técnico compatível; julgamento H0 cego |
| Ganho marginal | Por consulta depois da primeira: documentos novos, conceitos novos e consultas sem novidade |
| Próxima verificação | Top-3 contra `checks` |
| Proveniência | Trecho literal, com URL, hash e datas; verificado por código |
| Abstinência | Em `documented=false` ou sem evidência aplicável, declara insuficiência |
| Conclusões indevidas | Revisão H0 das hipóteses reportadas como sustentadas |
| Custo | Consultas, leituras, tempo, memória e requisições |
| Segurança | Testes e registro de rejeições |

**H9 favorável só se:** C supera B, B2 e F no conceito no relatório, com IC pareado acima de 0, sem piorar a proveniência nem as conclusões
indevidas. Vencer só A não basta. Vencer só em conceito lido também não. Resultado neutro ou negativo é publicado da mesma forma.

**Mesmo um resultado favorável não prova investigação autônoma completa:** o autor das regras também escreveu o gabarito, e nenhum
problema é verificado em um jogo real.

## Segurança

Conteúdo externo é dado, nunca instrução. Limites impostos no código, fora de qualquer decisão: lista de hosts e rotas, só GET, sem
destinos privados, redirecionamentos validados, tamanho e tempo limitados, orçamento, cache, limite de taxa. Termos vindos de documentos
só viram consulta se passarem por um formato restrito, sem qualificadores. Referências seguidas viram repositório + número em lista
permitida, e nunca uma URL copiada. Relatos ficam como citação e **nunca como instrução**: a próxima verificação sai só do catálogo próprio.
