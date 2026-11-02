# Protocolo do piloto do benchmark

Estado em 30/10/2026: **amostra sorteada; nenhuma anotação feita; sem anotadores independentes confirmados;
nenhum gabarito existe.** Decisão do responsável: preparar 30 a 50 casos. O piloto tem 40.

**Nada aqui é validação humana.** Resultados de agentes não entram neste benchmark.

## Objetivo

Medir se duas pessoas conseguem aplicar o [guia](ANNOTATION-GUIDE.md) de forma consistente. O piloto não mede
nenhum modelo. Serve para corrigir o guia, estimar o esforço e decidir se o benchmark completo é viável.

## Amostra

| Item | Valor |
|---|---|
| Fonte | Comentários públicos de `ValveSoftware/Proton`, no snapshot local do v7 |
| Exclusões | 491 threads já citados em dados rotulados ou reservados de v1–v7; membros do projeto e bots |
| Tamanho do comentário | 60 a 1500 caracteres |
| Unidade | Um comentário, no máximo um por thread |
| Semente | `frx-benchmark-pilot-20261030` |
| Hash de `packet/cases.jsonl` | `84760a641d41d141a68b24a9eb91fd14e291b4edf241a9a5a0f8ac69949284db` |

| Estrato | Casos | Filtro |
|---|---:|---|
| Vocabulário de resultado | 20 | Palavras amplas como *work*, *fix*, *crash*, *still* |
| Várias ações | 10 | Dois ou mais termos de ação ou opção |
| Sem filtro | 10 | Todo o pool |

O filtro só define estratos e não decide nada sobre um caso. As medidas por estrato devem declarar esse viés.

**O agente não leu os casos.** `pilot.py sample` só relata contagens. Os casos ficam em
`~/.cache/frx-ai-benchmark-pilot-20261030/packet/`, fora do Git, e o v8 exclui esses threads.

## Pessoas e níveis

| Papel | Requisito |
|---|---|
| Anotadores A e B | Duas pessoas, sem acesso às respostas uma da outra nem a previsões |
| Adjudicador | Uma terceira pessoa, ou o responsável se declarado |
| Registro | `annotators.json` na pasta do piloto, com `id`, `kind` e `independent_of_frx` |

| Nível | Definição | Texto permitido |
|---|---|---|
| H0 | Sem pessoa registrada | "Sem revisão humana" |
| H1 | Pelo menos uma pessoa, mas não duas independentes | "Revisão de uma pessoa" |
| H2 | Duas pessoas independentes do FRX, às cegas | "Avaliação independente" |

`agree` calcula o nível a partir do registro. Sem registro, o resultado é H0. Se o próprio responsável anotar,
o nível será H1, e ainda assim o piloto é útil para testar o guia. A ferramenta recusa a anotação dentro de uma
sessão de agente. Essa barreira evita enganos, mas não prova autoria: o que vale é a declaração de cada pessoa.

## Critérios de concordância, fixados antes de anotar

Cálculo com `pilot.py agree`. Implementação conferida nos exemplos publicados de Krippendorff (0,095 e 0,691).

| Campo | Métrica | Aceitável | Provisório | Revisar |
|---|---|---:|---:|---:|
| `overall` | Krippendorff α nominal | ≥ 0,80 | 0,667–0,80 | < 0,667 |
| Há afirmação (`has_claim`) | α | ≥ 0,80 | 0,667–0,80 | < 0,667 |
| `role` | α | ≥ 0,80 | 0,667–0,80 | < 0,667 |
| `relation` | α | ≥ 0,80 | 0,667–0,80 | < 0,667 |
| `evidence` | F1 de sobreposição | ≥ 0,80 | 0,60–0,80 | < 0,60 |

Também são relatados: κ de Cohen, IC de 95% por bootstrap, concordância bruta e matriz de confusão.

**Limite de 40 casos:** o intervalo é largo, e classes raras (`worsened`, `mixed`) podem aparecer poucas vezes. O
piloto indica problemas no guia, mas **não valida um campo com certeza.** Um campo com poucos exemplos é relatado
como não avaliável.

**Se ficar abaixo do limite:** revisar o guia e repetir em **casos novos**. Depois de duas revisões sem atingir o
limite, o campo é redefinido ou removido, e isso é relatado.

## Gabarito separado

| Local | Conteúdo | Quem acessa |
|---|---|---|
| `packet/` | Só `id`, título, URL e texto | Anotadores |
| `annotations/` | Respostas individuais | Coordenação, depois de concluídas |
| `sampling/` | Estratos e manifesto | Coordenação; não vai aos anotadores |
| `gold/pilot-gold.jsonl` | Gabarito, modo 600, criado só por `adjudicate` | Bloqueado para agentes antes da trava |
| `gold-lock.json` | Hash e nível de revisão | Público |

**Ordem obrigatória:**

1. Anotadores A e B trabalham separados.
2. `agree` mede a concordância.
3. O adjudicador decide as discordâncias.
4. `adjudicate` escreve o gabarito. Falta de decisão bloqueia a escrita.
5. Previsões de sistemas só contam se o hash foi travado **antes** do gabarito (`lock_predictions`).

## Esforço estimado

Cerca de 2 min por caso: 40 casos × 2 pessoas ≈ 2,7 h, mais a adjudicação. **Estimativa minha; não foi medida.**

## Comandos

```fish
python3 research/frx_ai/benchmark/pilot.py status
python3 research/frx_ai/benchmark/pilot.py annotate --annotator A
python3 research/frx_ai/benchmark/pilot.py agree
python3 research/frx_ai/benchmark/pilot.py adjudicate --decisions decisoes.jsonl
python3 -m unittest discover -s research/frx_ai/benchmark -p 'test_pilot.py'
```

Antes de começar, confira no pacote se há dados pessoais visíveis. O texto é público, mas não vai para o Git.

## Pendências

- Definir os anotadores.
- Uma pessoa deve ler o guia antes do início e apontar ambiguidades. Isso ainda não aconteceu.
