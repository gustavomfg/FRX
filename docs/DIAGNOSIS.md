# Diagnóstico por regras

A página de cada jogo Steam tem a **Central de Resolução**, com um botão **Diagnosticar**. Nada roda ao abrir a página. O diagnóstico consulta as fontes atuais e as receitas do FRX e mostra o que elas sustentam. **Ele não usa nenhum componente aprendido e não altera jogo, Steam ou prefixo.**

Esta versão substitui a Central retirada do código. Não restaura aquela implementação.

## Fluxo

1. **Contexto:** GPU e fabricante, driver, ferramenta Proton, arquivos conhecidos do jogo (como `nvngx_dlss.dll` e PhysX) e opções atuais de inicialização.
2. **Fontes:** os provedores atuais, com o cache e a validade configurados. Nenhuma fonte nova.
3. **Afirmações:** para cada fonte, `fixes/reports.rs` decide se o autor relata resultado, e qual. O trecho fica como citação literal.
4. **Mapeamento:** o Fix Engine diz se existe ação tipada. O que não mapeia fica **em revisão**, com o motivo.
5. **Deduplicação:** uma sugestão por configuração normalizada. Relatos contam por URL distinta. Um grupo de opções continua grupo, e o resultado não é creditado a um membro.
6. **Regras duras** e **veredito**.

## O que cada estado significa

| Estado | Significado | Prévia |
|---|---|---|
| Pronta para revisar | Evidência documentada ou vários relatos, e o planejador tem plano automático | Sim |
| Aguardando | Bloqueio temporário, como Steam aberta | Não |
| Já aplicada | Já está nas opções do jogo ou no histórico | Não |
| Já incluída no Proton | A ferramenta atual já aplica | Não |
| Em revisão | Um relato só, relatos divididos, sem resultado, sem ação segura ou jogo não confirmado | Não |
| Não sugerida | Incompatível, só relatos negativos ou finalidade errada para o sintoma | Não |

**Vereditos:** sugestões, só em revisão, sem evidência suficiente e fontes indisponíveis. Falha de consulta nunca aparece como "nada encontrado".

**Confiança** vem só do que as fontes afirmam: documentada, vários relatos (2+ fontes, mais favoráveis que contrárias), um relato, relatos divididos, sem relato de resultado. Não existe porcentagem. O nível indica sustentação nas fontes, não que a correção funcionará neste computador.

## Regras duras

- Upgrade de DLSS pelo Proton só com GPU NVIDIA **confirmada** e `nvngx_dlss.dll` no jogo, venha do catálogo ou de um comentário.
- Melhoria de imagem nunca é sugerida para crash ou falha ao iniciar. Sem sintoma informado, aparece separada, como melhoria opcional.
- Fonte que não confirma o jogo não gera sugestão aplicável.
- Texto de fonte nunca autoriza comando. Ações vêm dos campos tipados das descobertas. Valor com cara de comando fica em revisão, mesmo que algo o aceite.
- Aplicar abre a prévia do Fix Engine, que revalida tudo e exige confirmação. O diagnóstico não chama a aplicação.

## O que é determinístico

Todo o núcleo, em `src-tauri/src/diagnosis/`. `diagnose` é uma função pura: a mesma entrada dá o mesmo resultado, em qualquer ordem das fontes. **Nada é aprendido ou gerado por modelo.** A leitura de afirmações é feita por regras, com abstenção quando não há certeza.

## Limites

- Não forma hipótese de causa. Só resume o que as fontes relatam sobre configurações que o FRX conhece.
- As afirmações vêm do trecho salvo pelo provedor, e o comentário inteiro não é relido. Quando o provedor contou um relato e o trecho não o repete, a contagem só é usada sem ambiguidade.
- A leitura de relatos foi medida só com rótulos de agente, em inglês, sem revisão humana independente.
- Logs não são lidos, e o sintoma vem do seletor.
- Winetricks, registro, DLLs e troca de Proton ficam em revisão, por falta de ação reversível.
- O resultado não é salvo. Sempre é recalculado no clique.

## Testes

Nenhum faz rede ou toca na Steam real.

```fish
cd src-tauri
cargo test --lib diagnosis
cd ..
npx vitest run src/test/diagnosis.test.tsx
```
