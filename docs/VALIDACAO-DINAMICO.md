# Validação — Fix Engine dinâmico

Validação de 26/10/2026, em CachyOS com RTX 4060, driver 615.78.08, Steam aberta e Batman: Arkham City GOTY (200260) com proton-cachyos-11.0-20261015. Nenhum commit ou tag foi criado.

## Causa do bloqueio

Veja [`FIX-ENGINE-DINAMICO.md`](FIX-ENGINE-DINAMICO.md#o-bloqueio-que-existia-v040). Em resumo, havia três travas:

- o motor só aceitava receitas do catálogo embutido;
- o planejador só executava receitas `reviewed`;
- não existia conversão de descoberta em receita.

## Verificações automatizadas

| Comando | Resultado |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo test` | **180 aprovados**, 4 ignorados (antes: 161 aprovados, 3 ignorados) |
| `npm test` | **29 aprovados** em 7 arquivos (antes: 27) |
| `npm run build` | ok: AppImage (109,90 MiB), .deb e .rpm (4,13 MiB cada) |

### Testes novos (Rust)

`fix_engine/dynamic/tests.rs`, 13 testes:

- **Geração:** descoberta real `PROTON_NO_D3D11` vira receita experimental; várias ações viram uma receita com ID estável, independente da ordem; descoberta sem instruções; comentários contraditórios; mesma ação em fontes diferentes vira uma receita só (deduplicação); AppID errado, sem fonte, fonte não confiável e conteúdo malformado; ações não suportadas (winetricks, DLL, registro, arquivo, `gamemoderun`, `-dx11`, versão do Proton, variável desconhecida); tentativas de injeção (`;`, `&&`, `$(…)`, crases, `>`, `LD_PRELOAD`, `WINEDLLOVERRIDES`, `PATH`, camadas Vulkan, texto pedindo ao app para ignorar as regras); saída hostil de um interpretador futuro.
- **Validação:** valor fora do permitido; GPU incompatível; descoberta incompatível; já incluída no Proton; script condicional ou parcial; receita adulterada (12 casos recusados por `validate_generated`); catálogo recusa receitas geradas; `resolve` recusa quando a descoberta muda.
- **Registro:** reaproveita a análise sem regenerar, inclusive entre instâncias; reavalia e explica quando o Proton muda; arquivo corrompido é ignorado.

`fix_engine/tests.rs` (módulo `generated`), 6 testes de execução numa Steam falsa:

- descoberta → análise → prévia → backup → aplicação → reinício → histórico → Performance Lab (`active_for`) → rollback byte a byte;
- mudança da descoberta, da evidência ou das opções entre a prévia e a aplicação → recusa, sem gravar;
- cancelamento, conflito com receita revisada na mesma variável e “já incluída”;
- Steam aberta → passo manual → verificação;
- falha parcial → recuperação → rollback;
- registro persistente com permissão 0700.

Mais: `provenance` (só HTTPS do github.com, 1 teste) e `compatibility` (toda variável da base passa na allowlist, 1 teste).

### Testes novos (front-end)

`src/test/dynamicFix.test.tsx`, 2 testes:

- a interface envia só o `appId` para a análise;
- cada estado aparece com o motivo, sem a mensagem genérica “Receita indisponível”;
- **Aplicar correção** abre a prévia e não aplica;
- a prévia mostra “Por que foi sugerida”, o trecho original e os riscos;
- a confirmação tem texto próprio para receita gerada;
- a aplicação envia só IDs e a impressão digital;
- o histórico mostra “receita gerada automaticamente” e o cartão mostra “Correção aplicada · backup disponível”.

## Descobertas reais processadas

As descobertas reais vêm do cache do FRX (`~/.cache/io.frx.FRX/fixes/200260.json`), obtido do GitHub (ValveSoftware/Proton#309).

| Descoberta | Resultado | Evidência | Motivo |
|---|---|---|---|
| `PROTON_NO_D3D11=1 %command%` (comentário de 01/11/2018) | **Gerada** · `gen.200260.c6283a15fcef54c1` | um relato · experimental | Relatada com Proton 3.16; o jogo usa proton-cachyos-11 |
| `PROTON_USE_WINED3D=1` | Revisada do catálogo (`frx.compat.wined3d`) | contestada (0 × 2) | A contestação é exibida junto |
| `PROTON_USE_WINED3D11=1 %command%` | Requer validação | contestada | O FRX não conhece o efeito da variável |
| winetricks `d3dcompiler_43`, `d3dx9_43`, `dotnet20` | Requer validação | um relato | Baixa arquivos e executa instaladores |
| Mantenedor: “fixed in DXVK 1.4.1” | Informativa | documentada | Sem ação para aplicar |
| Issue #309 (relato geral) | Informativa | — | Sem ação técnica estruturada |

## Aplicação e rollback isolados

### 1. Teste com dados reais

Comando: `cargo test real_dynamic_end_to_end -- --ignored --nocapture`.

O teste lê o relatório real em cache e uma **cópia** do `localconfig.vdf` real (com a linha do MangoHud) dentro de uma Steam em pasta temporária. Resultado:

- o plano ficou pronto: uma linha que preserva as opções existentes e acrescenta `PROTON_NO_D3D11=1`;
- a correção foi aplicada;
- o rollback deixou o arquivo idêntico à cópia, byte a byte.

### 2. App Tauri (debug)

O app rodou com HOME, XDG, `FRX_STEAM_ROOT` e `FRX_DEV_PROC_ROOT` apontando para cópias no scratchpad (sem Steam aberta nesse `/proc` falso). Passos:

1. Fix Packs → Descobertas: o cartão do `PROTON_NO_D3D11` mostra “Correção aplicável”, “Receita gerada automaticamente”, “Experimental” e “Evidência: um relato”, com os três motivos e a variável marcada como reconhecida.
2. **Aplicar correção** abre a prévia: diff da linha, “Por que foi sugerida” com o trecho do comentário, efeito, risco e a verificação “Receita gerada”. O SHA do arquivo ficou igual.
3. Confirmação com o texto específico de receita gerada → resultado “Aplicada e verificada”. O arquivo isolado mudou; o hash do backup (`fix-engine/backups/op-…/0-localconfig.vdf`) coincidiu com o original; o diário registra `recipeTrust: generated` e `findingIds: [github-200260-0751180507]`.
4. Histórico → **Desfazer**: a prévia mostra que só remove `PROTON_NO_D3D11=1` → “Desfeita e verificada”. O arquivo voltou ao hash original.

O hash do `localconfig.vdf` real permaneceu igual antes e depois dos testes.

### 3. Release

O binário `target/release/frx` abriu a janela no ambiente isolado. Ele ignora `FRX_DEV_PROC_ROOT`, como antes.

## Limites desta validação

- Nada foi aplicado na Steam real (ela estava aberta, e alterar a instalação exige sua autorização).
- O jogo não foi aberto: o efeito do `PROTON_NO_D3D11` no Batman não foi testado. A receita é experimental por definição (um relato de 2018, Proton 3.16).
- A pesquisa não foi refeita na rede nesta validação; foi usado o relatório real já em cache.

## Teste manual na instalação real (quando você autorizar)

1. Feche a Steam (ou siga o modo manual com ela aberta).
2. Fix Packs → Descobertas → “Opção de inicialização sugerida” (`PROTON_NO_D3D11`) → **Aplicar correção** → confirme.
3. Abra o jogo. Se não abrir ou travar, desfaça pelo Histórico.
4. Para medir: capture uma sessão no Performance Lab antes e outra depois. A comparação mostra as correções ativas, sem atribuir causa.
