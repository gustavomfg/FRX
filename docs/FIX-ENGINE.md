# Fix Engine (v0.4.0)

O Fix Engine é o núcleo em Rust que aplica correções (revisadas do catálogo, geradas pelo gerador dinâmico — veja [`FIX-ENGINE-DINAMICO.md`](FIX-ENGINE-DINAMICO.md) — ou Smart Packs compostos, `pack.<appid>.<hash>`, aplicados como uma única operação — veja [`SMART-PACKS.md`](SMART-PACKS.md)) com prévia, backup verificado, gravação atômica, verificação no disco, diário persistente e rollback. Código em `src-tauri/src/fix_engine/`, comandos em `src-tauri/src/commands/fix_engine.rs` e interface em `src/components/fix/{ApplyFixDialog,RecipesPanel,OperationHistory}.tsx`.

## Do achado à operação

```
FixFinding (pesquisa v0.2, só leitura)
   │  mesmo ajuste? (relatedKeys)
   ▼
Receita candidata ──revisão humana──▶ Receita revisada (catálogo local, hash verificado)
                                          │  check / prepare (dry-run no Rust)
                                          ▼
                                     Plano (planId + fingerprint, expira em 10 min)
                                          │  confirmação do usuário
                                          ▼
                                     Operação (diário: checking → backing-up → applying → verifying → applied)
                                          │
                                          ▼
                                     Rollback (rolling-back → rolled-back) ou recuperação
```

- **Descobertas nunca executam.** Um arquivo do UMU-Protonfixes, uma issue do GitHub ou um comentário só aparecem como referência. Quando uma receita revisada trata do mesmo ajuste (por exemplo, `PROTON_USE_WINED3D=1`), o cartão da descoberta aponta para a receita, e é a receita que roda.
- **Confiança da receita:** `discovered`, `candidate`, `reviewed`, `unsupported` e `revoked`. Só `reviewed` vira plano executável. As demais aparecem como “Só consulta”.

## Formato da receita (schema 1)

As receitas são arquivos JSON em `src-tauri/recipes/`, embutidos no binário com `include_str!`. Campos desconhecidos são recusados (`deny_unknown_fields`).

```json
{
  "schema": 1,
  "id": "frx.diagnostics.proton-log",
  "version": "1.0.0",
  "trust": "reviewed",
  "title": "…", "summary": "…", "description": "…",
  "category": "other",
  "purpose": "diagnostic | compatibility | performance",
  "risk": "low | moderate | high",
  "scope": { "appIds": [], "requiresProton": true, "gpuVendor": null },
  "actions": [{ "type": "set_launch_env", "key": "PROTON_LOG", "value": "1" }],
  "alreadyIncludedByProtonfixes": false,
  "conflictsWith": [],
  "relatedKeys": ["env:PROTON_LOG=1"],
  "effects": ["…"], "sideEffects": ["…"],
  "sources": [{ "label": "…", "url": "https://…" }],
  "review": { "by": "FRX", "at": "2026-10-25", "notes": "…" }
}
```

### Ações (conjunto fechado)

| Tipo | O que faz | Limites |
| --- | --- | --- |
| `set_launch_env` | Põe `KEY=valor` no começo das opções de inicialização do jogo | Allowlist explícita de nomes e formatos de valor |
| `add_launch_args` | Acrescenta argumentos depois de `%command%` | Só opções simples (`-x`, `--x=y`), sem caminhos, aspas ou metacaracteres |
| `write_game_file` | Escreve um arquivo de texto **relativo** à pasta do jogo | Só `ini/cfg/conf/txt/json/xml`, até 256 KiB, sem `..`, sem links simbólicos, só dentro de `steamapps/common/` |

Não existem ações de comando, script, `execute_shell`, winetricks, caminho absoluto, download ou cópia de DLL. Um tipo desconhecido falha na leitura do JSON.

### Allowlist de variáveis

`PROTON_LOG=1`, `PROTON_NO_ESYNC=1`, `PROTON_NO_FSYNC=1`, `PROTON_NO_NTSYNC=1`, `PROTON_USE_WINED3D=1`, `PROTON_NO_D3D11=1`, `PROTON_NO_D3D10=1`, `PROTON_ENABLE_NVAPI=0|1`, `PROTON_HIDE_NVIDIA_GPU=0|1`, `PROTON_FORCE_LARGE_ADDRESS_AWARE=0|1`, `DXVK_FRAME_RATE`/`VKD3D_FRAME_RATE` (até 3 dígitos) e `DXVK_HUD` (lista `[a-z0-9,=.]`, até 64 caracteres).

O prefixo `PROTON_` não autoriza nada sozinho. `LD_*`, `*PATH*`, `*DLL*` (`WINEDLLOVERRIDES`), camadas Vulkan, `MANGOHUD_CONFIGFILE`, `STEAM_*`, `WINE*`, `HOME` e `SHELL` são recusados com mensagem própria. Valores com espaço, aspas, `;`, `$()`, crase ou quebra de linha são recusados.

### Catálogo e integridade

`recipes/catalog.json` lista cada arquivo com o SHA-256 esperado, a lista `revoked` e o campo `signature`, reservado para atualizações assinadas. Na abertura, o catálogo é validado:

- um arquivo com hash diferente, fora do manifesto, inválido ou com ID duplicado fica de fora e aparece em “Receitas recusadas pela verificação de integridade”;
- um ID em `revoked` vira `revoked` e não executa.

Hoje nada é baixado: o catálogo só muda com uma nova versão do FRX. Para adicionar uma receita, veja `src-tauri/recipes/README.md`.

### Catálogo atual

| Receita | Confiança | Ação | Observação |
| --- | --- | --- | --- |
| `frx.diagnostics.proton-log` 1.0.0 | Revisada | `PROTON_LOG=1` | Diagnóstico; não é otimização de desempenho |
| `frx.compat.no-esync-fsync` 1.0.0 | Revisada | `PROTON_NO_ESYNC=1` + `PROTON_NO_FSYNC=1` | Compatibilidade, risco moderado |
| `frx.compat.wined3d` 1.0.0 | Revisada | `PROTON_USE_WINED3D=1` | Compatibilidade, costuma reduzir o desempenho |
| `frx.compat.no-d3d11` 0.1.0 | Candidata | `PROTON_NO_D3D11=1` | Só consulta; conflita com `wined3d` |

Todas citam o README oficial do Proton (opções de execução).

## Planejamento (dry-run)

`planner::compute` lê o estado real e não grava nada. Verificações:

1. receita revisada; jogo no escopo; Proton em uso (`usesProton`, sem chutar quando é desconhecido); GPU exigida;
2. `alreadyIncludedByProtonfixes` com uma ferramenta que já traz o protonfixes;
3. histórico do mesmo jogo: a mesma receita já aplicada, `conflictsWith` nos dois sentidos e outra receita que já controla a mesma variável;
4. jogo aberto (`SteamAppId=<id>` no ambiente de algum processo) → conflito;
5. `localconfig.vdf` da conta mais recente: leitura estrita em UTF-8; arquivo inválido ou não encontrado → bloqueado;
6. linha atual interpretada pelo tokenizador compartilhado com o Performance Lab (`steam/launch_options.rs`):
   - **mesmo valor** já presente → “já aplicada”;
   - **valor diferente** → conflito, sem sobrescrever;
   - variável depois de um wrapper, operadores de shell ou `%command%` duplicado → conflito;
7. Steam aberta (`/proc/*/comm` `steam`/`steamwebhelper` ou `ubuntu12_32/steam`) → `requires-manual-step`.

**Estados do plano:** `ready`, `requires-manual-step`, `conflict`, `incompatible`, `already-applied`, `already-included` e `not-executable`.

A **impressão digital** é o SHA-256 de receita + versão + hash do catálogo + AppID + passos concretos (inclui o SHA-256 do `localconfig.vdf` e a linha atual) + estado da Steam e do jogo + ferramenta. O front-end recebe `planId` e `fingerprint` e devolve só isso ao confirmar (`apply_fix_plan(planId, fingerprint, appId)`).

## Execução transacional

`FixEngine::apply` segue esta ordem:

1. trava global de execução (`try_lock`: uma escrita por vez; a segunda recebe “Aguarde”);
2. recupera o plano guardado (uso único, expira em 10 min) e o **recalcula**: se a impressão digital mudou, nada é gravado;
3. **modo manual** (Steam aberta): registra a operação como `requires-manual-step` com a linha a colar e não grava nada;
4. `backing-up`: copia cada alvo para `<app_data>/fix-engine/backups/<operação>/<n>-<arquivo>` com `O_EXCL` e modo 0600, faz `fsync`, relê e confere o SHA-256. A ausência do arquivo original também é registrada. Antes da cópia, confere o espaço livre (`statvfs`). Se o backup falhar ou o hash não bater com o do plano, a operação aborta antes de qualquer escrita;
5. `applying`: confere de novo se o jogo ou a Steam abriram, relê o arquivo e exige o mesmo SHA-256 do plano. Troca **só** a fatia do valor `LaunchOptions` daquele AppID (`steam/localconfig.rs`, edição posicional: o resto do arquivo fica byte a byte igual) e grava de forma atômica: temporário exclusivo no mesmo diretório, permissões do original, `fsync`, `rename` e `fsync` do diretório. Links simbólicos no destino são recusados;
6. `verifying`: relê o arquivo, confere o SHA-256 gravado e o valor interpretado;
7. `applied`. Se algo falhar depois de gravar, o backup é restaurado (somente se o arquivo ainda for o que o FRX escreveu). A operação vira `failed`, ou `recovery-required` quando nem isso é seguro.

Cada transição vai para o diário (`<app_data>/fix-engine/operations/<id>.json`, escrita atômica) **antes** da etapa seguinte e é emitida no evento `fix-engine://progress`, que a interface usa para mostrar o progresso real.

## Integração com a Steam

O FRX **nunca** encerra a Steam, mata processos ou interrompe jogos.

| Situação | Comportamento |
| --- | --- |
| Steam fechada, jogo fechado | Modo automático: backup, edição posicional, verificação |
| Steam aberta | Modo manual: mostra a linha para colar em Propriedades › Geral › Opções de inicialização. A operação fica “Aguardando passo manual” e só vira `applied` depois de **Conferir agora** encontrar a linha no `localconfig.vdf`. A Steam grava o arquivo ao fechar |
| Jogo aberto | Conflito: feche o jogo |
| Linha atual com operadores, wrapper sem `%command%` ou VDF ilegível | Conflito, sem linha sugerida |
| Edição posicional impossível (estrutura inesperada) | Só o modo manual |

“Aplicada” significa “conferida no disco”. Em **Correções aplicadas**, o estado é relido a cada consulta: `active` (ainda presente), `drifted` (alterada fora do FRX) ou `unknown` (ilegível).

## Rollback

`prepare_fix_rollback(operationId)` → prévia; `rollback_fix(operationId, fingerprint)` → execução.

- O rollback **não** restaura o backup inteiro: isso apagaria mudanças legítimas posteriores, como tempo de jogo e opções de outros jogos. Ele remove só o que a operação adicionou: cada variável volta ao valor anterior (ou é removida) **apenas se ainda tiver o valor que o FRX escreveu**, e os argumentos adicionados são removidos.
- Valor alterado por outro programa → conflito, sem escrita. A interface mostra o motivo e não oferece “Desfazer”.
- Antes de gravar, o rollback também faz backup (`rollback-<n>-<arquivo>`), confere o hash logo antes da escrita e verifica depois.
- Com a Steam aberta, o rollback também é manual e conferido no disco.
- Arquivos de jogo: são restaurados do backup (ou removidos, se o FRX os criou) somente se o conteúdo atual for o que o FRX gravou.
- Uma operação manual ainda não colada pode ser cancelada sem alterar nada (estado `failed`, “Cancelada antes de ser aplicada”).

## Recuperação

Na abertura, `FixEngine::recover` examina operações em estados intermediários:

- nenhum passo gravado e arquivos iguais ao backup → `failed` (“fechado antes de gravar; nada mudou”);
- caso contrário → `recovery-required`, com um aviso no topo de Fix Packs. Nada disso conta como aplicado.

A partir daí, **Conferir estado real** (`verify_fix_operation`) marca como `applied` se a configuração estiver completa, ou como `failed` se nada ficou. **Desfazer** usa o rollback seguro.

## Integração com o Performance Lab

- Sessões da captura assistida guardam `activeFixes` (receita, versão, operação e título) — só correções `applied` **confirmadas agora no disco**. Importações e sessões antigas ficam com `null` (“Não registrado”), nunca com uma lista vazia inventada.
- A comparação lista “Correções do FRX ativas” como diferença **não crítica** e acrescenta a nota de que a diferença de desempenho não prova causalidade. As estatísticas não mudaram.
- O tokenizador de opções de inicialização é o mesmo do Performance Lab (`steam/launch_options.rs`). A linha do MangoHud (`MANGOHUD_CONFIGFILE=… mangohud %command%`) é preservada pelo Fix Engine, e a do Performance Lab continua compatível com variáveis inseridas pelo Fix Engine.

## Comandos Tauri

| Comando | Escreve? | Descrição |
| --- | --- | --- |
| `get_fix_engine_status` | não | Catálogo, problemas de integridade, recuperações e passos manuais pendentes |
| `list_fix_recipes(appId?)` | não | Receitas + achados relacionados do último relatório |
| `get_fix_recipe(recipeId)` | não | Uma receita do catálogo |
| `analyze_fix_findings(appId)` | não | Gerador dinâmico: análise e receitas geradas das descobertas em cache |
| `check_fix_compatibility(recipeId, appId)` | não | Plano calculado sem guardar |
| `prepare_fix_plan(recipeId, appId)` | não | Plano guardado em memória para confirmação |
| `apply_fix_plan(planId, fingerprint, appId)` | **sim** | Executa o plano confirmado |
| `get_fix_operation`, `list_fix_operations`, `get_applied_fixes` | não | Diário e estado real |
| `prepare_fix_rollback(operationId)` | não | Prévia do rollback |
| `rollback_fix(operationId, fingerprint)` | **sim** | Executa o rollback confirmado |
| `verify_fix_operation(operationId)` | só o diário | Confere um passo manual ou uma recuperação |

Não há API genérica de escrita de arquivos nem plugin `fs`. As capabilities não mudaram.

## Desenvolvimento e testes

- `cargo test fix_engine` executa o fluxo completo contra uma raiz da Steam falsa (tempdir), com um `/proc` falso para simular a Steam ou o jogo abertos. Nada toca a instalação real.
- **Somente em builds de debug**, `FRX_DEV_PROC_ROOT=/pasta` troca a raiz do `/proc`, para validar o modo automático contra uma Steam isolada enquanto a Steam real está aberta. O binário de release ignora essa variável.
- Para montar uma Steam isolada a partir de cópias, veja `docs/VALIDACAO-v0.4.0.md`.
