# Guia de anotação — piloto do benchmark, versão 1

Para pessoas. Leia antes de abrir qualquer caso. Os exemplos abaixo foram inventados para este guia.

**Regras:**

- Anote só com o texto do caso. Não use modelo, LLM, tradutor automático nem as respostas de outra pessoa.
- Não abra o link para ler o resto da conversa: o caso é só o comentário e o título. Se isso impedir a decisão, use `unclear`.
- Na dúvida real, use `unclear`. Não é erro, é informação.
- Não infira a causa. Você marca o que a pessoa **afirma**, não o que é verdade.

## O que marcar em cada caso

### 1. Papel (`role`)

| Valor | Quando |
|---|---|
| `first_hand` | A pessoa relata o que aconteceu com ela |
| `second_hand` | Relata o que outros disseram ou fizeram: "people report that…" |
| `request_or_question` | O trecho principal pede ou pergunta |
| `advice` | Recomenda algo sem dizer que testou |
| `information` | Explica ou dá contexto, sem relato próprio |
| `unclear` | Não dá para decidir |

### 2. Resultado geral (`overall`)

**Afirmação de resultado:** a pessoa diz qual foi o efeito sobre o problema de uma **mudança ou tentativa**: uma
opção, uma versão, um arquivo, uma atualização.

| Valor | Significado | Exemplo inventado |
|---|---|---|
| `resolved` | O problema deixou de ocorrer | "Set `OPTION_X=1` and the crash is gone" |
| `improved` | Melhorou, sem resolver | "Downgrading helped a bit, but it still stutters sometimes" |
| `not_resolved` | Nada mudou, ou a tentativa falhou | "Tried Proton 9, same black screen" |
| `worsened` | Piorou ou passou a falhar depois da mudança | "After the update it no longer launches" |
| `mixed` | Efeitos opostos sem um geral: uma coisa melhorou e outra piorou, ou duas tentativas com resultados opostos | "Option A fixed audio, but option B broke the menu" |
| `no_claim` | Não há efeito de mudança ou tentativa | Ver abaixo |
| `unclear` | Pode haver uma afirmação, mas não dá para decidir | |

**É `no_claim`:**

- Sintoma sem tentativa: "The game crashes on launch".
- Funciona sem nenhuma mudança relatada: "Works out of the box".
- Hipóteses e planos: "This might fix it", "I'll try X tomorrow".
- Perguntas: "Did that fix it for you?"

**Decisões difíceis:**

- "Works with Proton 8" sem falar de um problema: `resolved` só se o texto indicar que a pessoa testou essa
  configuração contra um problema. Sem isso, `no_claim`.
- Agradecimento sem efeito ("thanks, will try"): `no_claim`.
- Citação de outra pessoa (linhas com `>`) não conta como afirmação do autor. Anote apenas o que o autor escreveu.
- Se o autor relata o efeito na máquina de outra pessoa: mantenha o resultado e marque `second_hand`.
- Vários resultados na mesma direção: use a classe comum.

### 3. Evidência (`evidence`)

Copie o **menor trecho literal** que sustenta o resultado, um por afirmação. Obrigatória em toda afirmação de
resultado. A ferramenta recusa trechos que não existam no texto.

### 4. Ações (`actions`)

Copie um trecho literal por mudança ou tentativa ligada ao resultado, sem incluir sintomas ou contexto.

### 5. Relação entre ações (`relation`)

| Valor | Quando | Exemplo inventado |
|---|---|---|
| `single` | Uma ação | "Disabling X fixed it" |
| `joint` | Feitas **juntas**, com um resultado comum, sem saber qual importou | "`A=1 B=1 %command%` works" |
| `alternatives` | Tentativas **separadas**, cada uma com o resultado | "Tried A, B and C, none helped" |
| `sequence` | Ordem explícita, com o resultado ao final | "First did A, then B, then it launched" |
| `unclear` | Há mais de uma ação, mas a relação não é decidível | |
| `not_applicable` | Sem ações ligadas a um resultado | |

Não atribua o efeito a um único membro de um grupo, a menos que o autor o faça.

## Como usar a ferramenta

```fish
python3 research/frx_ai/benchmark/pilot.py annotate --annotator A
```

A sessão pode ser interrompida e retomada. Cada pessoa usa sua própria letra e não vê as respostas da outra.
Ao final, a ferramenta pede a declaração de que não houve assistência de modelo ou LLM.
