# FRX AI v8 — protocolo fixado antes dos dados

Data: 30/10/2026. Objetivo: aproximar a FRX AI de investigar problemas reais, e não de produzir classificações mais
sofisticadas.

## Limites

- **Nenhum treino.** Só regras e pesos congelados como comparadores. Um teste garante que o código do v8 não chama
  `.fit` nem importa treinadores.
- **Rótulos do v8 são H0:** escritos por este agente (Claude Opus 5.5), só para avaliação. **Não são validação
  humana** e não entram no benchmark.
- O piloto do benchmark fica de fora: seus threads são excluídos, e os casos não são lidos nem pontuados.
- **Logs:** só fixtures escritas aqui e trechos colados em comentários públicos já presentes no snapshot, redigidos
  e fora do Git. Sem baixar anexos. Nenhum arquivo privado: a CLI recusa caminhos fora de `fixtures/`.
- Nada é executado a partir de texto recuperado. Não altera aplicativo, Steam, jogos ou prefixos.

## Por que redefinir a tarefa

Medido sozinho nos gabaritos antigos, o detector do v7 tem cobertura de 49–86% e precisão de 42–69%. Mas esses
gabaritos só marcam resultados ligados a uma ação: muitos "falsos positivos" são sintomas reais. **Portanto os
gabaritos antigos não medem a precisão de um detector isolado.** O v8 separa:

- **Estado:** "the game crashes". Descreve a situação.
- **Afirmação de resultado:** efeito de uma mudança ou tentativa, conforme o [guia](../benchmark/ANNOTATION-GUIDE.md).

## Frentes e hipóteses

**O — Detecção isolada de resultados.** Sem gerador de eventos nem associação.

- H-O1: separar estado de resultado por regras explícitas reduz os falsos resultados, sem perder cobertura, em relação
  às regras do v7.
- H-O2: pesos congelados não superam as regras nessa tarefa. Se a interface deles não permitir comparar, isso é relatado.

**J — Procedimentos conjuntos.** O v7 recuperou 2 de 20 grupos.

- H-J1: o problema principal é de representação. Um grupo descrito por **conjunto de intervenções + relação + resultado**
  expressa os casos do J7 que a saída do v7 não consegue expressar.
- H-J2: sinais sintáticos simples, como opções na mesma linha de inicialização, recuperam mais grupos que o v7.
- H-J3: atribuição individual falsa permanece em zero.

**T — Trajetórias completas.** Esquema: problema → evidência → hipótese → verificação → resultado → decisão seguinte.

- H-T1: escolher a verificação que mais separa as hipóteses abertas supera "sempre pedir o log" e as regras do v7 em
  replay incremental de threads reais.
- H-T2: o ciclo muda de decisão quando recebe um resultado, e não afirma causa sem suporte.

**L — Logs.** H-L1: as regras encontram o sinal relevante melhor que uma busca literal e não inventam causa.

## Coortes

Todas excluem threads de v1–v7 e do piloto. Uma unidade por thread em O e J.

| Coorte | Tamanho | Uso | Quando é sorteada |
|---|---:|---|---|
| `o8-dev` | 30 | Escrever regras; erros podem ser lidos | Antes das regras |
| `o8-test` | 60 (40 + 20 com várias ações) | Medição única | **Depois do congelamento** |
| `t8-dev` | 4 threads | Escrever a política | Antes da política |
| `t8-test` | 12 threads | Medição única | **Depois do congelamento** |
| `l8` | 8 trechos de log + fixtures | Medição, com trechos sorteados depois do congelamento | **Depois do congelamento** |

Ordem do teste: congelar código por hash → sortear → rotular e travar por hash, sem ver previsões → rodar uma vez.
Correções posteriores só valem como replay conhecido, separadas da primeira leitura.

## Critérios, fixados agora

Com rótulos H0 e n pequeno, um resultado favorável autoriza **levar o sistema à avaliação humana**, e nada além disso.

| Frente | Métrica primária | Favorável se |
|---|---|---|
| O | Falsos `resolved`; precisão de `resolved`; cobertura de afirmações | Falsos `resolved` menores que v7, e cobertura não menor que v7 − 5 pontos |
| O | Acerto de classe no nível da unidade, pareado com v7 | IC bootstrap de 95% da diferença acima de 0 |
| J | Grupos com relação e membros corretos | Mais grupos que v7 na mesma coorte; atribuição individual falsa = 0 |
| T | Top-3 contra verificações aceitáveis, por ponto de decisão | IC pareado por thread acima de 0, contra o melhor comparador |
| T | Causa afirmada sem suporte | 0 |
| T | Pedido do que já foi informado | Menor ou igual às regras do v7 |
| L | Sinais relevantes achados; causa inventada | Maior que a busca literal; causa inventada = 0 |

**Definições fixadas antes do teste:**

- O v7 só tem uma classe positiva. Na comparação entre sistemas, `resolved` e `improved` viram **positivo**, e o critério
  "falsos `resolved`" compara **afirmações positivas falsas** no nível da unidade. A separação fina do v8 é relatada à parte.
- Casos que o rótulo H0 marca `unclear` ficam fora dos erros. Um positivo sobre gabarito `mixed` não conta como falso.
- Comparadores: v7 (regras sobre parser), `weak` do v2 (regex), E07 do v2 (pesos congelados: `e07` pelo critério seletivo daquele
  laboratório, `e07_any` sem ele) e sempre `no_claim`. **`reports.rs` e o `outcome` do v3 ficam fora:** ambos exigem a ação
  já dada, e esta tarefa não a fornece. Os dois ficam para o benchmark humano.
- Grupo correto: mesma relação e todos os membros cobertos, com no máximo um membro extra.

Resultado neutro ou desfavorável é publicado da mesma forma. **Mesmo o resultado favorável não é diagnóstico inteligente:
tudo aqui é regra.**

## Origem de cada capacidade

Toda saída e relatório separa:

| Origem | No v8 |
|---|---|
| **Regra** | Detector, grupos, hipóteses, escolha de verificação, sinais de log |
| **Recuperação de evidências** | TF-IDF sobre comentários públicos; recuperador R02 congelado só com opção explícita |
| **Aprendido** | Nada novo. Pesos antigos só como comparadores, sem treino |

## Simulador

Cenários sintéticos testam a **lógica** do ciclo, não a linguagem nem o diagnóstico. Como cenários e conhecimento têm o mesmo
autor, o resultado é circular e **não entra nos critérios acima**.
