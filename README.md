# FRX — Play. Fix. Repeat.

Protótipo de front-end do **FRX**, um gerenciador de compatibilidade e otimização de jogos para Linux (Steam, Proton e Wine).

> **Modo demonstrativo.** Tudo nesta etapa é simulado: jogos, estúdios, hardware, problemas, fontes, correções e benchmarks são fictícios. Nenhum arquivo é lido ou alterado, nada é baixado e nenhum comando é executado.

## Como executar

O FRX é um **aplicativo desktop Linux** feito com **Tauri 2** (Rust) e uma interface em React + TypeScript + Vite.

### Dependências de compilação

- Node 20+ (testado com Node 24) e npm
- Rust estável (testado com 1.97) via [rustup](https://rustup.rs)
- Bibliotecas do sistema para o WebKitGTK:

| Distribuição | Comando |
| --- | --- |
| Arch / CachyOS / Manjaro | `sudo pacman -S --needed webkit2gtk-4.1 gtk3 base-devel curl wget file openssl librsvg` |
| Debian / Ubuntu | `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev build-essential curl wget file libssl-dev librsvg2-dev` |
| Fedora | `sudo dnf install webkit2gtk4.1-devel gtk3-devel openssl-devel curl wget file librsvg2-devel && sudo dnf group install c-development` |

Opcionais: `libappindicator-gtk3` (bandeja do sistema, ainda não usada) e `fuse2` (para abrir o AppImage sem extraí-lo; sem ele, use `APPIMAGE_EXTRACT_AND_RUN=1`).

### Comandos

```bash
npm install
npm run dev            # abre a janela do FRX (Tauri + Vite com recarga ao vivo)
npm run build          # binário de release + AppImage, .deb e .rpm
npm run build:appimage # só o AppImage
npm run build:arch     # pacote nativo do pacman (Arch/CachyOS)
npm run dev:web        # apenas a interface no navegador, em http://localhost:1420
npm run typecheck
```

Onde ficam os artefatos:

```
src-tauri/target/release/frx                                   binário
src-tauri/target/release/bundle/appimage/FRX_0.1.0_amd64.AppImage
src-tauri/target/release/bundle/deb/FRX_0.1.0_amd64.deb
src-tauri/target/release/bundle/rpm/FRX-0.1.0-1.x86_64.rpm
packaging/arch/frx-bin-0.1.0-1-x86_64.pkg.tar.zst             (sudo pacman -U …)
```

Na build de release, a interface (`dist/`) é **embutida no binário** e servida pelo protocolo interno do Tauri. Não é preciso um servidor Vite. O Vite só é usado em `npm run dev`, pela `devUrl` `http://localhost:1420`.

### Integração com o desktop

- Janela nativa com título "FRX", 1440×900, tamanho mínimo de 1024×700 e fundo `#101011` (sem flash branco ao abrir).
- O app id e a classe da janela são `frx`. Os arquivos `.desktop` usam `StartupWMClass=frx`, então o ícone aparece certo na barra de tarefas, tanto em Wayland quanto em X11.
- O ícone vem de `packaging/icon.svg` (o mesmo símbolo d-pad da interface). Para regenerar, rode `npx tauri icon packaging/icon.png -o src-tauri/icons`.
- **Gerenciadores de janela em mosaico** (Hyprland, Sway…) podem ignorar o tamanho mínimo. Para abrir o FRX flutuando, adicione uma regra para a classe `frx` no seu WM.
- **WebKitGTK + NVIDIA no Wayland:** o app define `WEBKIT_DISABLE_DMABUF_RENDERER=1` ao iniciar, a menos que você já tenha definido essa variável. Isso evita a janela em branco conhecida nessa combinação.

### Ponte TypeScript ↔ Rust

- **Rust:** `src-tauri/src/commands.rs` expõe os comandos `app_info` (versão, kernel, distro e sessão, só leitura) e `ping`.
- **TypeScript:** `src/services/platform.ts` chama esses comandos com `invoke` quando roda dentro do Tauri. No navegador, cai em um substituto demonstrativo.
- **Na interface:** o resultado aparece em **Configurações → Sobre → Execução**.
- **Permissões:** `src-tauri/capabilities/default.json` libera só o núcleo do Tauri. A CSP restringe scripts, estilos, fontes e imagens aos arquivos do próprio app.

As futuras funções nativas (detecção de jogos, aplicação de correções, benchmarks) devem entrar como novos comandos em `commands.rs`, com funções correspondentes em `platform.ts`.

### Atalhos

| Atalho | Ação |
| --- | --- |
| `Ctrl + K` | Busca rápida (páginas, jogos, Fix Packs) |
| `Alt + 1…5` | Ir para Início, Biblioteca, Fix Packs, Performance Lab, Configurações |
| `/` | Focar a busca da Biblioteca |
| `Esc` | Fechar diálogos (exceto durante uma operação simulada) |

## Stack

Tauri 2 (Rust) · React 19 · TypeScript · Vite · Tailwind CSS 4 · Motion · Recharts · Lucide. As fontes (Bricolage Grotesque, Geist, Geist Mono e Silkscreen) vêm do Fontsource e são empacotadas localmente, sem depender de rede.

## Estrutura

```
src-tauri/                camada nativa (Rust): lib.rs, commands.rs, tauri.conf.json, capabilities/, icons/
packaging/               ícone-fonte (SVG/PNG) e pacote Arch (PKGBUILD + .desktop)
src/
  app/routes.ts            navegação principal
  layouts/                 AppShell, Sidebar, CommandPalette, Page
  pages/                   Home, Library, GameDetail, FixPacks, Performance, Settings, NotFound
  components/
    ui/                    Button, controles, Dialog, Toasts, status, contadores, estados vazios
    game/                  GameArt (capas pixeladas geradas), GameCard/GameRow
    fix/                   FixFlowDialog (aplicar/reverter), ChangeList, fontes
    perf/                  CompareHero, MetricChart, vereditos
    brand/                 Logo (símbolo d-pad + wordmark)
  design/tokens.css        design tokens (cores, raios, movimento, acentos)
  mascot/                  Pim: sprites procedurais, relógio compartilhado, componente
  data/                    dados demonstrativos (jogos, problemas, fix packs, benchmarks…)
  services/                demo.ts (simulações com latência) e platform.ts (ponte futura)
  state/                   store (reducer + contexto), preferências de movimento, seletores
  types/                   tipos de domínio
```

## Identidade visual

- **Tokens:** todas as cores vivem em `src/design/tokens.css` e são expostas ao Tailwind via `@theme inline` (`bg-surface`, `text-fg-2`, `text-accent`…). Os acentos alternativos (`menta`, `coral`, `ceu`) já funcionam em Configurações; novos temas só precisam redefinir as variáveis sob `[data-theme='…']`.
- **Símbolo:** um direcional (d-pad) em pixel art que também é um "+" de primeiros socorros: jogar e consertar no mesmo gesto.
- **Pim:** o mascote original, um marshmallow de gorro amarelo-manteiga. Ele é desenhado proceduralmente numa grade de 24×20 (`mascot/sprites.ts`) e renderizado como SVG nítido (um `<path>` por cor). Tem 10 poses: acenando, descansando, investigando com a lupa, consertando com a chave, observando, comemorando, preocupado, em dúvida e outras.
- **Capas:** cada jogo fictício recebe uma cena pixelada gerada em SVG a partir de uma semente fixa (`GameArt.tsx`), sem assets de terceiros.

## Movimento e acessibilidade

- Um único relógio de 8 Hz anima todos os sprites e pausa quando a janela fica oculta ou quando nenhum Pim está visível.
- Intensidade **Completa / Suave / Desligada**, além de "Seguir o sistema / Sempre reduzir" para `prefers-reduced-motion`. O nível efetivo vai para `<html data-motion>` e para o `MotionConfig`.
- Foco visível em todos os controles, link "Pular para o conteúdo", controles segmentados navegáveis por setas, diálogos com foco preso, `aria-live` nos progressos.
- Status de compatibilidade, risco e severidade nunca dependem só da cor: sempre vêm com glifo pixelado e texto.
- Nos gráficos, "antes" é tracejado e "depois" é sólido, com legenda e tooltip.

## O que é simulado

| Área | Comportamento atual | Futuro (Tauri) |
| --- | --- | --- |
| Biblioteca | 12 jogos fictícios em `data/games.ts` | Detecção em Steam/GOG/Epic/Heroic |
| Sistema | Perfil fixo em `data/system.ts` | Leitura real de GPU, driver, kernel e Proton |
| Análise | Etapas com atraso (`services/demo.ts`) | Consulta a ProtonDB, GitHub, UMU-Protonfixes |
| Fix Packs | Aplicar e reverter só alteram o estado da interface; um pacote (anti-cheat do Ashen Choir) falha de propósito na primeira tentativa para demonstrar o estado de erro | Escrita real com backup e rollback |
| Benchmarks | Amostras geradas por PRNG; o veredito considera a variação entre passadas (melhora, regressão, estável, inconclusivo) | Captura via MangoHud ou similar |
| Sensores | Temperatura e consumo simulados | Dependem do suporte do driver |
| Persistência | Só as preferências (localStorage). Correções e benchmarks voltam ao estado inicial ao recarregar ou em "Reiniciar dados da demo" | Banco local |

O app usa `HashRouter` e caminhos relativos (`base: './'`) para funcionar empacotado.
