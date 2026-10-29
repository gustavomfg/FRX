# Avaliação da FRX AI com modelos locais

Data: 2026-10-29. Branch `feat/frx-ai-local`. Sem integração ao aplicativo e sem modelo nos instaladores.

## Objetivo
Verificar se um modelo local pequeno consegue interpretar relatos reais de Crimson Desert (AppID 3321460) sobre opções de Proton: qual opção foi usada, se foi ativada ou desativada e qual foi o resultado.

## Ambiente
- llama.cpp v0.8.0, compilado só para CPU (sem CUDA, Vulkan, OpenSSL ou OpenMP). Binário `llama-completion`, dependências apenas da libc e da libstdc++.
- Modelos, todos Apache-2.0, com SHA-256 conferido:
  - Qwen3-0.6B Q4_K_M (unsloth): 396.705.472 bytes, `ac2d9771…`
  - Qwen3-1.7B Q4_K_M (unsloth): 1.107.409.472 bytes, `b139949c…`
  - Qwen3-4B Q4_K_M (Qwen oficial): 2.497.280.256 bytes, `7485fe6f…`
- Dados: 233 textos de comentários e notas em cache; 50 citam uma variável de Proton, VKD3D ou DXVK.

## Resultados

| Modelo | Casos rotulados (6) | Lote de 30 textos | Caso `descriptor_heap` |
|---|---|---|---|
| 0,6B | Respondeu `resolved` quase sempre; índices inválidos | Não avaliado (já reprovado) | Errado |
| 1,7B | Acertou 2 de 4 com resultado; 2 armadilhas corretas | 15 relatos; 5 falsos resultados a partir de linhas de comando; 1 ação errada | Não identificado |
| 4B | Acertou 2 com resultado; 2 armadilhas corretas | 52 relatos; 16 "resolvido/melhorou"; pelo menos 7 erros graves | Não identificado (`unknown`), saída inválida no lote |

## Erros observados
- Linha de comando tratada como resultado.
- Versão do Proton ou instalação de biblioteca tratada como opção.
- Estado (ativado ou desativado) trocado.
- "Melhorou" sem base no texto.
- Nota de correção lida como "não resolveu".
- Regressão (geometria distorcida após `descriptor_heap`) não identificada.

## Decisão
Nenhum modelo atingiu a confiabilidade exigida. Nada foi integrado ao FRX, e nenhum modelo entrou nos pacotes. Mantida a alternativa segura: citações originais, com link, sem classificar resultado.

## Limites
Amostra pequena, com rótulos e revisão feitos manualmente a partir do texto. Não mede desempenho de jogo.

## Rodada 2: classificadores pequenos (2026-10-29)
- Dados: 185 frases rotuladas manualmente, a partir de comentários em cache. São 120 amostradas de 246 que têm vocabulário de resultado, mais 65 com nomes de opções. Distribuição: sem resultado 101, negativo 48, positivo 29, piora 7.
- Regressão logística em numpy, com palavras e pares de palavras, validação cruzada de 5 partes: acurácia 0,60 (chute por maioria: 0,55). Não reconheceu nenhuma piora e errou o caso `descriptor_heap`.
- Regras explícitas: acurácia 0,63 no próprio conjunto usado para escrevê-las, portanto otimista. Positivos com precisão de 0,57: 2 de 37 eram negativos. O caso `descriptor_heap` saiu como piora, corretamente.
- Conclusão: nenhum dos dois é confiável o bastante para afirmar um resultado. Nada foi integrado.

## Rodada 3: extrator de relatos por opção, integrado (2026-10-29)

### Por que as rodadas anteriores falharam
- A unidade estava errada. O FRX e as rodadas 1–2 julgavam o comentário ou a frase inteira, e o FRX atribuía o mesmo positivo ou negativo a todas as opções citadas. Um comentário que diz "X resolveu, Y piorou, tente Z" virava três votos positivos.
- Os rótulos da rodada 2 eram frases soltas. Sem o contexto do bloco de código, não dá para saber de qual opção a frase fala.
- As regras da rodada 2 foram medidas no mesmo conjunto usado para escrevê-las.

### Abordagem
`src-tauri/src/fixes/reports.rs`, em Rust puro, sem modelo e sem dependência nova. Para cada par (comentário, opção), o extrator segue estes passos:
1. Descarta citações (`>`) e o modelo de issue (`<!-- -->`).
2. Liga a opção à sua frase: a própria frase, ou, para um bloco de código, a frase que o introduz.
3. Separa sugestão e pergunta de experiência própria.
4. Decide o resultado com escopo de negação e contraste.
5. Se abstém quando o resultado é atribuído a outra causa (versão, regressão, downgrade) ou quando os resultados se contradizem.

O resultado é resolveu, não resolveu, piorou ou misto. Também registra se a opção foi desligada, quais opções foram usadas junto e a frase que sustenta o resultado. Se abster não conta como erro: só reduz a cobertura.

### Avaliação
- Dados reais de issues do ValveSoftware/Proton, separados por issue, para que comentários do mesmo tópico não vazem entre os conjuntos:
  - desenvolvimento: Crimson Desert e cache de Batman;
  - teste A: 8 issues, 80 pares, com erros inspecionados uma vez;
  - teste B: 2 issues, 40 pares, intocado até a medição final.
- Critério fixado antes de medir: precisão ≥ 0,90 no teste B e nenhum "resolveu" falso.
- Os rótulos de A e B foram refeitos a partir do comentário completo, antes de abrir as previsões de B. Os primeiros rótulos tinham sido feitos com trechos de 420 caracteres, e 4 "erros" do teste A eram, na verdade, erros de rótulo (A06, A24, A58, A61). O extrator não foi alterado depois dessa correção.

| Conjunto | Método | Afirmações | Corretas | Precisão | Relatos reais encontrados | "Resolveu" falsos |
|---|---|---|---|---|---|---|
| A | FRX anterior (palavras no comentário inteiro) | 42 | 12 | 0,29 | 12 de 23 | 20 |
| A | FRX AI | 15 | 15 | 1,00 | 15 de 23 | 0 |
| B | FRX anterior | 17 | 2 | 0,12 | 2 de 16 | 10 |
| B | FRX AI | 9 | 9 | 1,00 | 9 de 16 | 0 |

Os relatos não encontrados são abstenções, e nenhum virou erro:
- teste B: 2 de 6 "resolveu", 4 de 8 "não resolveu" e 1 de 2 "misto";
- teste A: 4 de 12 "resolveu", 3 de 6 "não resolveu" e 1 de 3 "misto". Os 2 "piorou" foram encontrados.

O caso `descriptor_heap` do Crimson Desert sai como "piorou".

### Limites
- Os rótulos são meus, feitos às cegas em relação às previsões, mas sem um segundo anotador. Amostra pequena: 24 afirmações no teste. Com 9 de 9 no B, o intervalo de confiança de 95% da precisão vai de cerca de 0,66 a 1,0.
- Cobertura de 56% a 65%. Relatos escritos de forma indireta ficam sem resultado.
- Avaliado só com variáveis de ambiente, em inglês. Verbos do winetricks usam o mesmo extrator, sem avaliação própria. Linhas de comando completas e configurações não recebem resultado.

### Integração
`providers/github.rs::collect` agora usa `reports::analyze` para cada opção, no lugar do tom do comentário inteiro:
- "resolveu" vira evidência positiva;
- "não resolveu" ou "piorou" vira evidência negativa;
- misto, sugestão ou abstenção não contam.

Um relato feito com a opção desligada não conta para quem a liga. A citação exibida passa a ser a frase que sustenta o resultado. Essa evidência alimenta `Evidence`, a validação e o Fix Engine, que mantêm as regras de compatibilidade e de confirmação do usuário.
