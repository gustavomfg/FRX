# Validação do FRX v0.3.0

Data: 24/10/2026. Repositório: `/home/g/Documentos/FRX`. Último commit ao assumir: `936f803`.

## Ponto de partida

A implementação interrompida já continha os oito módulos Rust do Performance Lab, os comandos Tauri registrados, os providers React, a interface real, os sprites novos e os testes recém-criados em `src/test/performance.test.tsx`. O trabalho existente foi continuado, sem recriar esses módulos ou redesenhar a interface.

Na primeira execução: **114 testes Rust aprovados, 3 ignorados e 16 testes React aprovados**; TypeScript, formatação e Clippy também passaram. Não havia `AGENTS.md` ou `CLAUDE.md` no repositório ou nas pastas ancestrais verificadas. As alterações pendentes foram preservadas; nenhum commit foi criado nesta retomada.

## Correções realizadas

- Mesclagem das opções de inicialização por posições das palavras, preservando aspas, escapes e espaços nos argumentos. Caminhos gerados recebem proteção para o shell. Linhas ambíguas, operadores e substituições de comandos são recusados com orientação de importação manual.
- Preparação, início, cancelamento e publicação final da captura serializados. Uma preparação concorrente é recusada; processamento cancelado não publica uma sessão oculta.
- Cancelar ou fechar o FRX envia a parada ao MangoHud e libera o socket. A configuração só é armada quando o monitor começa a observar.
- Mensagens do socket remontadas quando chegam em partes. A conexão só é considerada confirmada após o handshake do MangoHud. Há timeouts para ausência de gravação, falta de confirmação e logs parados.
- Leitura incremental de logs sem drenar/copiar o restante da string para cada linha; tratamento de linhas parciais, truncamento e limite de tamanho. A descoberta de capturas ignora links simbólicos.
- Configurações idênticas deixam de ser regravadas. Isso corrigiu a repetição com HUD oculto no mesmo processo: substituir o arquivo desnecessariamente acionava o hot reload do MangoHud e interrompia o controle.
- Importação limitada também durante a leitura, cobrindo arquivos que crescem depois da checagem inicial de tamanho. O processo de detecção é recolhido após timeout.
- Histórico serializado entre threads; metadados publicados depois do log e das séries. Reprocessamento preserva os avisos de interrupção.
- Classificação de duração tolera 0,1 s nos limites de 10/30 s: o MangoHud pode finalizar uma captura entre quadros. Os valores medidos permanecem intactos. Metodologia incrementada para `analysisVersion: 4`, com reprocessamento do histórico existente.
- Estado real limpo ao entrar na demonstração e operações nativas bloqueadas nesse modo. Edição atualiza o detalhe carregado e permite remover a associação ao jogo.
- Valores ausentes continuam como “—” na comparação, em vez de virar zero.
- Detalhe do jogo real ligado ao Performance Lab por **Medir desempenho**. Resumo da Home e texto de Configurações atualizados para os benchmarks reais.
- Ajustes pontuais de quebra do cabeçalho e rolagem horizontal da tabela, preservando tipografia, cores, componentes e mascotes existentes.
- README atualizado para a versão, comandos, metodologia e primeiro benchmark, mantendo o Pim de boas-vindas e a captura já existente.

## Verificações automatizadas

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Aprovado |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Aprovado, sem avisos |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --quiet` | **123 aprovados, 0 falhas, 3 ignorados** |
| `npm test` | **18 aprovados, 0 falhas**, em 5 arquivos |
| `npm run typecheck` | Aprovado |
| `npm run build` | Binário Linux, AppImage, `.deb` e `.rpm` gerados |
| `bash -n packaging/arch/PKGBUILD` | Aprovado |
| `git diff --check` | Aprovado |

Os três testes Rust ignorados são diagnósticos manuais de varredura real, consulta externa e importação de um log escolhido por variável de ambiente. A suíte normal preserva os testes da v0.2. O build emite um aviso não bloqueante do Vite para um chunk de aproximadamente 606 kB.

O **AppImage final foi aberto sem Vite**, com `APPIMAGE_EXTRACT_AND_RUN=1`. A interface de produção carregou, detectou o MangoHud e mostrou o histórico persistido, incluindo a segunda captura consecutiva. Os pacotes `.deb` e `.rpm` foram gerados, mas não instalados neste sistema Arch.

As chaves serializadas pelo Rust foram comparadas com os contratos TypeScript em dez objetos reais: `Session`, `SessionMeta`, `Conditions`, `Quality`, `SessionStats`, `FpsStats`, `FrametimeStats`, `Stability`, `CaptureStatus` e `CapturePlan`.

## Execução real

Ambiente observado: CachyOS, Hyprland/Wayland, NVIDIA GeForce RTX 4060, Steam nativa e MangoHud **0.8.4-1+**, com camadas de 32/64 bits detectadas.

O app Tauri foi aberto de verdade. O inspetor WebKit e a acessibilidade GTK foram usados para dirigir o aplicativo existente; os comandos Rust e o controle do MangoHud não foram substituídos por mocks. Dados e cache de validação ficaram em diretórios XDG isolados sob `output/playwright/v03-audit/`. A Steam foi apenas lida.

Capturas medidas com `mangohud vkcube`, usando a configuração gerada pelo FRX:

| Cenário | Evidência |
| --- | --- |
| Automático, HUD oculto | 1.509 quadros, 9,99 s, 151,01 FPS médios; sessão salva |
| Automático, HUD visível | 1.777 quadros, 10,00 s, 177,77 FPS médios; sessão salva |
| Início e parada manuais pelo controle | 780 quadros, 4,51 s; sessão curta sinalizada corretamente |
| Encerramento abrupto do `vkcube` | 591 quadros, 3,52 s; sessão salva com aviso de interrupção |
| Cancelamento | Nenhuma nova sessão no histórico |
| Duas capturas no mesmo `vkcube`, sem fechá-lo, HUD oculto | 1.788 e 1.771 quadros em CSVs distintos; duas sessões salvas |
| Fechar o FRX durante uma gravação | Jogo permaneceu aberto; log parou, resumo foi escrito e nenhuma sessão nova foi salva |

Também passaram: importação pelo seletor nativo GTK, gráfico com dados reais, histórico, seleção Antes/Depois, comparação, renomeação refletida no detalhe, exclusão pelo diálogo, exportação JSON/CSV, persistência após reabrir e reprocessamento da sessão anterior. Resumo `_summary.csv`, duplicação de log e caminho especial foram recusados com mensagens específicas. Duas preparações simultâneas produziram uma sessão preparada e uma recusa, sem criar benchmarks adicionais.

Biblioteca Steam real → detalhe do jogo → **Medir desempenho** abriu o filtro correto. Configurações, modo demonstrativo, isolamento dos dados reais e mascote parado com animações desligadas foram exercitados. A comparação foi conferida em **1440×900 e 1024×700**, com gráficos renderizados e sem transbordamento horizontal da página.

Evidências locais, logs e capturas da validação permanecem em `output/playwright/v03-audit/`, que já está ignorado pelo Git.

A revisão do controle e da recarga de configurações também consultou os arquivos oficiais [`control.cpp`](https://github.com/flightlessmango/MangoHud/blob/v0.8.4/src/control.cpp) e [`notify.cpp`](https://github.com/flightlessmango/MangoHud/blob/v0.8.4/src/notify.cpp) do MangoHud 0.8.4.

## Limites da validação

- As medições acima são do `vkcube`. Não alterei launch options da Steam nem executei um jogo Steam com opções modificadas.
- Steam Flatpak, OpenGL, jogos de 32 bits e execução por `gamescope` não foram validados ponta a ponta. A montagem das opções com wrappers e aspas tem cobertura unitária.
- Prepare a captura com o jogo fechado. Para mudar duração/HUD, reabra o jogo com a nova configuração. A repetição sem fechar foi confirmada com a mesma configuração e HUD oculto.
- Fix Packs continuam sem aplicação real.

A implementação está funcional no cenário Linux/Vulkan validado. A validação em um jogo específico pela Steam depende da colagem manual da linha proposta pelo usuário, conforme o fluxo documentado no README.

## Arquivos tocados nesta retomada

O `git status` também contém arquivos que já estavam pendentes antes desta sessão. Os ajustes desta retomada estão nestes arquivos:

- `README.md` e `docs/VALIDACAO-v0.3.0.md`.
- `src-tauri/src/lib.rs`, `src-tauri/src/commands/performance.rs`.
- `src-tauri/src/performance/{capture,detection,mod,models,metrics,persistence}.rs`.
- `src/state/performance.tsx`.
- `src/pages/{RealPerformance,RealHome,Settings,SteamGameDetail}.tsx`.
- `src/components/performance/{CaptureFlow,ComparisonView,HistoryTable}.tsx`.
- `src/test/{perfFixtures.ts,performance.test.tsx}`.

## Primeiro benchmark

Abra `npm run dev`, entre em **Biblioteca → jogo → Medir desempenho → Nova sessão**, prepare com o jogo fechado, confira e cole a linha nas opções de inicialização da Steam, clique em **Já colei — aguardar o jogo** e abra-o. Use 60 s ou mais no mesmo trecho para comparações úteis. Alternativamente, **Importar benchmark** aceita um CSV principal já gravado pelo MangoHud.
