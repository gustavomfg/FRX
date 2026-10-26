# Validação do FRX v0.4.0 — Fix Engine

Data: 25/10/2026. Repositório: diretório do projeto FRX. Último commit no início: `7ec79e3` (árvore limpa). Nenhum commit ou tag foi criado; todas as mudanças estão no diretório de trabalho para revisão.

## Ponto de partida

Antes de mexer, a base passou nas verificações: `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, **123 testes Rust aprovados (3 ignorados)**, **18 testes do front-end** e typecheck. Não havia `AGENTS.md` nem `CLAUDE.md` para atualizar.

## O que foi feito

- **Núcleo Rust** (`src-tauri/src/fix_engine/`): modelos, catálogo embutido com manifesto SHA-256 e lista de revogação, regras de segurança (allowlist de variáveis, argumentos, caminhos confinados), planejador (dry-run, conflitos, impressão digital), backup verificado, escrita atômica, diário persistente, verificação, rollback seletivo e recuperação na abertura.
- **Steam** (`src-tauri/src/steam/`):
  - `launch_options.rs` reúne o tokenizador que antes estava em `performance/capture.rs`, agora compartilhado com o Performance Lab;
  - `localconfig.rs` edita `LaunchOptions` de forma posicional, alterando só a fatia do valor;
  - `processes.rs` detecta a Steam e o jogo abertos pelo `/proc`, só lendo.
- **12 comandos Tauri** (`commands/fix_engine.rs`) e o evento `fix-engine://progress`. As capabilities não mudaram, e não há plugin `fs`.
- **Catálogo:** 3 receitas revisadas (`proton-log`, `no-esync-fsync`, `wined3d`) e 1 candidata (`no-d3d11`), todas baseadas no README oficial do Proton.
- **Interface:**
  - Fix Packs ganhou as visões **Descobertas / Receitas do FRX / Histórico**, com o diálogo real de prévia, confirmação, progresso, modo manual e rollback, e um aviso de operações pendentes ou em recuperação;
  - o cartão da descoberta aponta para a receita revisada correspondente;
  - a página do jogo ganhou a seção **Correções do FRX**;
  - sem redesign: os mesmos componentes, tokens, Pim e animações.
- **Performance Lab:** as sessões assistidas registram `activeFixes`, e a comparação lista diferenças de correções sem atribuir causa.
- Versão **0.4.0** em `Cargo.toml`, `package.json`, `package-lock.json` (raiz), `APP_VERSION`, Sidebar e `PKGBUILD`.

## Verificações automatizadas

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets -- -D warnings` | OK, sem avisos |
| `cargo test` | **159 aprovados**, 0 falhas, 3 ignorados (`real_fix_search`, `real_log` e `real_system_scan`, que dependem da rede ou do sistema real) |
| `npm test` (Vitest) | **27 aprovados** em 6 arquivos |
| `npm run build:web` (tsc + Vite) | OK. O aviso de chunk acima de 600 kB já existia |
| `npm run build` | `frx` de release, `FRX_0.4.0_amd64.AppImage` (109,8 MiB), `.deb` (4,05 MiB) e `.rpm` (4,05 MiB) |
| `npm run build:arch` | `frx-bin-0.4.0-1-x86_64.pkg.tar.zst` |

### Testes novos

**Rust (+36):**

- `fix_engine` (23):
  - catálogo íntegro; arquivo adulterado ou fora do manifesto; receita revogada; `execute_shell`; `LD_PRELOAD`; campo extra;
  - allowlist (o prefixo `PROTON_` não basta; metacaracteres); argumentos; caminhos (`..`, absolutos, `.dll`/`.exe`); links simbólicos que escapam da pasta do jogo; prefixo recusado como pasta do jogo;
  - escrita atômica preservando o modo; backup e restauração com hash; backup adulterado recusado;
  - aplicar → verificar → rollback devolvendo **o arquivo byte a byte**; prévia desatualizada ou impressão digital forjada recusadas sem escrever;
  - rollback que preserva MangoHud adicionado pelo usuário e mudanças da Steam em outro jogo; rollback recusado quando o valor foi alterado (estado `drifted`);
  - Steam aberta → modo manual sem escrita, conferência real e rollback manual; cancelamento de operação manual pendente;
  - regras de compatibilidade (candidata, nativo, Proton desconhecido, jogo aberto, mesmo valor/valor diferente, linha com `;`, sem `localconfig`);
  - conflitos (`conflictsWith` nos dois sentidos, mesma variável); argumentos adicionados e removidos exatamente;
  - arquivos de jogo: criar/remover, substituir/restaurar, conflito de terceiros, link simbólico, pasta fora de `steamapps/common`;
  - operações concorrentes serializadas; Steam abrindo entre a prévia e a gravação; operação interrompida → `recovery-required` ou `failed` sem nunca aparecer como aplicada;
  - descobertas ligadas às receitas;
- `steam::localconfig` (5): leitura; troca só da fatia; ida e volta idêntica; inserção de chave ou bloco; escape de aspas e recusa de controle; estrutura desconhecida;
- `steam::processes` (2);
- `steam::launch_options` (5): definir e reverter variáveis na linha real do usuário, preservar MangoHud, gamescope e aspas, recusar linhas inseguras, adicionar e remover argumentos;
- `performance::comparison` (+1): correções diferentes listadas sem bloquear e com a nota de não causalidade.

**Front-end (+9, `src/test/fixEngine.test.tsx`):** estado do núcleo; listagem aplicável × candidata; prévia → confirmação → progresso por evento → aplicada, com a chamada contendo **só** `planId`, `fingerprint` e `appId`; falha do núcleo exibida sem “aplicada”; modo manual com linha e conferência; incompatível sem botão de aplicar; histórico → rollback (só `operationId` e `fingerprint`); conflito de rollback sem “Desfazer”; demonstração sem chamadas ao Rust; comparação com correções diferentes.

## Execução real no app

### 1. Steam isolada (debug, modo automático)

Montei uma raiz da Steam isolada em um diretório temporário, com **cópias** de `loginusers.vdf`, `config.vdf`, `localconfig.vdf`, `appmanifest_200260.acf` e da versão do prefixo, além de `HOME`/`XDG_*` próprios e `FRX_DEV_PROC_ROOT` apontando para um `/proc` vazio (válido só em debug). A Steam real continuou aberta e intocada.

| Passo | Resultado observado |
| --- | --- |
| Pesquisa de correções | 5 descobertas relevantes; WineD3D e `PROTON_NO_D3D11` ligadas às receitas (“citada em 1 descoberta”) |
| Receitas do FRX | 3 “Aplicável” e 1 “Candidata”. As verificações mostram o Proton real (`proton-cachyos-11.0-20261015`) e as opções atuais reais (omitidas por privacidade) |
| Prévia `proton-log` | Diff `−`/`+` com `PROTON_LOG=1` no início, preservando MangoHud e as variáveis; nota de backup |
| Aplicar | Etapas `checking → backing-up → applying → verifying → applied` no diário; Pim comemorando |
| Disco | `diff` mostra **uma única linha** alterada (linha 1195). Backup `0-localconfig.vdf` com SHA-256 igual ao original; pastas `fix-engine/`, `backups/` e `operations/` em 0700; permissões do `localconfig.vdf` preservadas |
| Rollback | Prévia “remove só o que esta correção adicionou”. Depois, `cmp` mostrou o arquivo **idêntico ao original**; `rollback-0-localconfig.vdf` guardado |
| Steam “aberta” (entrada falsa no `/proc` isolado) | Todas as revisadas como “Aplicável · passo manual”. A operação ficou “Aguardando passo manual” e nada foi gravado. “Conferir agora” antes de colar manteve o estado; depois da linha colada (simulando a gravação da Steam ao fechar), passou a `applied`, modo manual |
| Interrupção simulada (diário em `applying` com passo gravado) | Na reabertura: “Requer recuperação” e aviso no topo; nada aparece como aplicado. “Desfazer” devolveu o arquivo idêntico ao original |
| Página do jogo | Seção “Correções do FRX” com o histórico e “3 receitas revisadas aplicáveis agora” |

### 2. Steam real (debug, somente leitura)

Com `HOME` e `/proc` reais e os dados do FRX em uma pasta temporária, as receitas de Batman: Arkham City GOTY (200260) apareceram como **“Aplicável · passo manual”**: a Steam real está aberta. O app leu as opções reais e **nenhuma aplicação foi feita**. Nenhuma operação foi criada, e a pasta de dados real (`~/.local/share/io.frx.FRX`) continua sem `fix-engine/`.

O `localconfig.vdf` real mudou durante a sessão, mas só em `"AppInfoChangeNumber"` (valor omitido), um contador que a própria Steam aberta regravou. `LaunchOptions` está intacto e não há `PROTON_LOG` no arquivo. Isso confirma, na prática, que a Steam reescreve esse arquivo enquanto roda — por isso o FRX não grava com ela aberta.

### 3. Pacotes de release

- **Binário de release** (Steam isolada, `FRX_DEV_PROC_ROOT` definido): ignorou a variável, enxergou a Steam real pelo `/proc` verdadeiro e ofereceu só o modo manual. A interface está embutida e a barra lateral mostra `v0.4`.
- **AppImage:** abre a janela normalmente.

## Ajustes feitos durante a validação

- Ordem estável na lista de receitas, para a seleção não pular depois de aplicar.
- Foco do teclado levado ao controle principal a cada etapa do diálogo (a caixa de confirmação recebe o foco).
- Título “Conferir …” para operações pendentes; o texto do resultado segue o estado real da operação.
- Rótulo “Pode ser desfeita” na prévia do rollback.
- Seleção automática quando há um único jogo da Steam.
- Etiqueta do cabeçalho por visão.
- Coluna de data do histórico sem quebra.
- Pastas do Fix Engine criadas com 0700 desde o início.
- Configurações › Sobre deixou de dizer “sem alterar nada no sistema”; a confirmação extra e a retenção de backups ficaram identificadas como opções da demonstração, porque o Fix Engine sempre confirma e guarda o backup de cada operação. Os pacotes foram gerados de novo depois disso.

## Limites desta validação

- **Não apliquei nenhuma correção na instalação real da Steam:** ela estava aberta, e encerrá-la exige autorização. O caminho automático foi validado só contra cópias.
- O modo manual foi exercitado com uma entrada falsa no `/proc` e com a “colagem” simulada no arquivo. A interação real na Steam (colar, fechar a Steam e conferir) segue o roteiro abaixo.
- O jogo não foi aberto, portanto não houve teste de efeito em execução (por exemplo, a geração de `~/steam-200260.log`) nem captura assistida com `activeFixes`. O registro está coberto por testes de unidade e pela comparação.
- `write_game_file` está implementado e testado, mas nenhuma receita revisada o usa.

## Teste manual na instalação real (quando você autorizar)

1. Abra o FRX (`npm run dev` ou o pacote), vá em **Fix Packs → Receitas do FRX** e escolha Batman: Arkham City GOTY.
2. **Modo manual (Steam aberta):** escolha “Log de diagnóstico do Proton” → **Revisar e aplicar** → **Continuar no modo manual** → marque a confirmação → **Registrar e mostrar a linha**. Copie a linha, cole em Steam › Propriedades › Geral › Opções de inicialização, **feche a Steam você mesmo** e clique em **Conferir agora**. O estado deve virar “Aplicada”.
3. **Modo automático:** feche a Steam e o jogo, abra a mesma receita e confirme a prévia. O FRX cria o backup em `~/.local/share/io.frx.FRX/fix-engine/backups/<operação>/`, grava, confere e mostra “Aplicada”. Abra a Steam e confira as opções do jogo.
4. Abra o jogo uma vez. O Proton deve criar `~/steam-200260.log`.
5. **Rollback:** em **Histórico**, clique em **Desfazer**. Com a Steam fechada, o FRX remove só `PROTON_LOG=1`; com ela aberta, mostra a linha sem a variável para colar e conferir.
6. Opcional: depois de aplicar, mude `PROTON_LOG=1` para `PROTON_LOG=0` na Steam e tente desfazer. O FRX deve recusar, com “O FRX não sobrescreve mudanças de outros programas”.

## Comandos para repetir a validação isolada

```sh
# cópias, nunca os originais
S=/tmp/frx-iso; mkdir -p $S/home/.local/share/Steam/{config,steamapps,userdata/<conta>/config} \
  $S/lib/steamapps/common/"<pasta do jogo>" $S/lib/steamapps/compatdata/<appid>/pfx $S/proc
cp ~/.local/share/Steam/config/{loginusers,config}.vdf $S/home/.local/share/Steam/config/
cp ~/.local/share/Steam/userdata/<conta>/config/localconfig.vdf $S/home/.local/share/Steam/userdata/<conta>/config/
cp <biblioteca>/steamapps/appmanifest_<appid>.acf $S/lib/steamapps/
# libraryfolders.vdf da cópia apontando para $S/home/.local/share/Steam e $S/lib
npm run dev:web &
cd src-tauri && cargo build && HOME=$S/home XDG_DATA_HOME=$S/xdg XDG_CACHE_HOME=$S/cache \
  FRX_DEV_PROC_ROOT=$S/proc ./target/debug/frx
```
