<p align="center">
  <img src="docs/assets/pim.gif" width="144" height="120" alt="Pim acenando para dar boas-vindas ao Hardware Fixer">
  <br>
  <strong>Olá! Bem-vindo ao Hardware Fixer.</strong>
</p>

# Hardware Fixer (FRX)

[English](README.md)

**Hardware Fixer** é um app desktop para **consertar hardware de laptops Linux**: Wi-Fi, áudio, touchpad, suspensão e firmware. Ele identifica o modelo do laptop pelo DMI, pesquisa soluções na **Arch Wiki** para aquele modelo e, quando a página traz uma instrução clara (uma opção de módulo, um parâmetro do kernel), mostra uma prévia exata da mudança em `/etc`, com backup e desfazer.

> **O FRX nunca roda `sudo`.** Para mexer em `/etc`, ele faz o backup, prepara o arquivo numa pasta privada e mostra o comando para você rodar no terminal. O comando confere, como root, que o arquivo não mudou desde a prévia antes de sobrescrever. Depois, o FRX confere o hash do resultado em `/etc`. O mesmo vale para desfazer.

O projeto nasceu como um gerenciador de correções de jogos (FRX — *Play. Fix. Repeat.*). O motor em Rust — Fix Engine, provedores, cache, detecção do sistema — foi mantido e agora atende ao hardware. As telas de jogos continuam no repositório, mas fora das rotas (ver [Legado](#legado-jogos)).

## Como funciona

1. **Identificação.** Fabricante, modelo e placa vêm de `/sys/devices/virtual/dmi/id` (sem privilégios; número de série e UUID nunca são lidos). Textos genéricos de firmware ("To Be Filled By O.E.M.", "System Product Name"…) contam como ausentes. Na Lenovo, o nome comercial vem de `product_version` (`product_name` é o código do tipo de máquina).
2. **Sintomas.** A tela inicial mostra o seu sistema (modelo · GPU · kernel) e cinco sintomas: Wi-Fi, Áudio, Touchpad, Suspensão e Firmware.
3. **Busca na Arch Wiki.** Pela API MediaWiki pública, no máximo duas requisições com cache: a busca pelo modelo e o texto da página encontrada. A página só é aceita se todos os números do modelo baterem ("Gen 9" nunca vira "Gen 10"). Do texto saem as seções do sintoma, o status da tabela de hardware ("Funciona segundo a wiki") e sempre a página geral do sintoma. Conteúdo da Arch Wiki sob GFDL 1.3+, mostrado como citação com link.
4. **Instruções aplicáveis.** Um parser estrito reconhece só instruções inequívocas: `options <módulo> <chave=valor>` e parâmetros do kernel num trecho que manda aplicá-los. Frases negativas ("não funciona"), comandos e nomes de serviço são descartados. Instruções com condições ("só em Intel", "se…") aparecem marcadas e exigem confirmação de leitura.
5. **Prévia e aplicação.** "Aplicar correção" abre a prévia do Fix Engine: o arquivo em `/etc`, o conteúdo exato, verificações, backup, efeitos (reinício, initramfs) e o comando `sudo`. Depois de rodar o comando, "Já rodei — conferir" verifica o hash. As correções ficam em **Correções no sistema**, com "Desfazer".

### Correções de sistema suportadas

| Ação | Arquivo | Observações |
| --- | --- | --- |
| Opção de módulo | `/etc/modprobe.d/frx-<módulo>.conf` | Arquivo novo do FRX; avisa se outro arquivo já configura o módulo |
| Serviço do systemd | `/etc/systemd/system/frx-<módulo>-sleep.service` | Modelo fixo (recarregar o módulo ao suspender); sem `ExecStart` livre. Ainda não gerado pelo parser |
| Parâmetros do kernel | `/etc/default/grub` | Altera só a linha `GRUB_CMDLINE_LINUX_DEFAULT`; risco alto. Sem GRUB, o card mostra "Incompatível" |

Parâmetros que mudam o alvo do boot ou desligam proteções (`init=`, `rd.break`, `mitigations=`, `selinux=`…) nunca são aplicados.

## Segurança

- **Allowlist fechada de escrita.** O `confined.rs` só abre para escrita `/etc/modprobe.d/frx-*.conf`, `/etc/systemd/system/frx-*.service` e `/etc/default/grub`. Cada diretório é aberto com `O_NOFOLLOW`; links simbólicos, hard links, arquivos especiais, traversal e diretórios trocados entre a prévia e a escrita são recusados.
- **Sem `sudo` automático, sem `pkexec`, sem `sed -i`.** O FRX não executa nenhum comando privilegiado. Comandos que sobrescrevem ou removem um arquivo existente passam por `sudo sh -c` com `sha256sum --check` contra o hash lido na prévia: se o arquivo mudou, nada é alterado. Isso fecha a janela entre mostrar o comando e você rodá-lo.
- **Conferência no disco.** A correção só conta como aplicada (ou desfeita) depois que o FRX confere o hash em `/etc`. Uma correção pendente pode ser cancelada pelo próprio diálogo sem tocar no sistema.
- **A interface só envia IDs.** Sintoma, sugestão e ação; o núcleo refaz a busca pelo cache, recalcula a prévia e compara a impressão digital antes de qualquer coisa.
- **Rede restrita.** HTTPS para uma lista fechada de hosts (inclusive em redirecionamentos), tamanho de resposta limitado, cache com ETag. Conteúdo externo é texto, nunca HTML nem código.
- Backups, diário e arquivos preparados ficam em `~/.local/share/io.frx.FRX/fix-engine/` (diretórios 0700). Cache em `~/.cache/io.frx.FRX/`.

## Como executar

App desktop **Tauri 2** (Rust) com interface React + TypeScript + Vite.

### Dependências de compilação

- Node 20.19+ ou 22.12+ e npm
- Rust **1.88+** via [rustup](https://rustup.rs)
- Bibliotecas do WebKitGTK:

| Distribuição | Comando |
| --- | --- |
| Arch/CachyOS/Manjaro | `sudo pacman -S --needed webkit2gtk-4.1 gtk3 base-devel curl wget file openssl librsvg` |
| Debian/Ubuntu | `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev build-essential curl wget file libssl-dev librsvg2-dev` |
| Fedora | `sudo dnf install webkit2gtk4.1-devel gtk3-devel openssl-devel curl wget file librsvg2-devel && sudo dnf group install c-development` |

### Comandos

```bash
npm install
npm run dev            # abre a janela (Tauri + Vite com recarga ao vivo)
npm run build          # binário de release + AppImage, .deb e .rpm
npm run build:arch     # pacote nativo do pacman (Arch/CachyOS)
npm run build:web      # confere o TypeScript e compila a interface
npm test               # testes do front-end (Vitest)
(cd src-tauri && cargo test)                         # testes do Rust (fixtures, sem rede)
(cd src-tauri && cargo clippy --all-targets -- -D warnings)
(cd src-tauri && cargo test -- --ignored)            # testes ao vivo: Arch Wiki e leitura do /etc real
```

No navegador (`npm run dev:web`) não há acesso ao sistema; a interface pede para abrir o app desktop.

### Testar num desktop (fixture de desenvolvimento)

Só em build de debug, a tela inicial mostra **"Injetar laptop de teste (dev)"**. Ele simula apenas a identidade DMI; a busca vai à Arch Wiki de verdade e o Fix Engine é o real (em modo manual, sem gravar em `/etc`).

- **Framework Laptop 13:** tem instruções aplicáveis. Em **Wi-Fi**, `options iwlwifi disable_11ax=Y`; em **Suspensão**, `nvme.noacpi=1` (que aparece "Incompatível" numa máquina sem GRUB).
- **ThinkPad X1 Carbon Gen 9:** tem página na wiki, mas nenhuma instrução aplicável nas seções de sintomas.
- "Voltar ao DMI real" (ou reiniciar o app) desfaz a simulação.

### Integração com o desktop

- O id do app e a classe da janela são `frx` (`StartupWMClass=frx` nos `.desktop`).
- Em gerenciadores lado a lado (Hyprland, Sway…), adicione uma regra para a classe `frx` se quiser a janela flutuando.
- **WebKitGTK + NVIDIA no Wayland:** o app define `WEBKIT_DISABLE_DMABUF_RENDERER=1` ao abrir, salvo se você já definiu a variável.

## Ponte TypeScript ↔ Rust

| Comando | Para que serve |
| --- | --- |
| `identify_laptop` | Fabricante, modelo, placa e nome de exibição pelo DMI (ou o laptop de teste, em debug) |
| `get_system_profile` | Distribuição, kernel, sessão, CPU, RAM, GPUs e drivers |
| `search_hardware_fixes` | Busca na Arch Wiki para um sintoma, com a checagem leve de cada ação |
| `prepare_system_fix`, `apply_system_fix` | Prévia de uma instrução e confirmação (só IDs e a impressão digital da prévia) |
| `list_fix_operations`, `get_fix_operation`, `verify_fix_operation` | Histórico e conferência no disco |
| `prepare_fix_rollback`, `rollback_fix` | Prévia e execução do desfazer (ou cancelamento de uma correção pendente) |
| `get_fix_providers`, `clear_fix_cache`, `get_cache_size` | Fontes e cache |
| `set_dev_laptop` | Só em debug: laptop de teste |
| `app_info`, `ping` | Informações do app e teste da ponte |

Os comandos de jogos (Steam, Performance Lab, Smart Packs) continuam registrados, sem uso na interface atual.

## Estrutura

```
src-tauri/src/
  system/                  dmi (identidade do laptop), hardware (GPUs), context
  fixes/providers/         arch_wiki (busca + parser de instruções), fwupd (esqueleto, desativado);
                           legado de jogos: umu, github, artifacts, protondb, reddit, winehq, steam_guides
  fix_engine/              confined (allowlist e escrita presa a descritores), system (ações de sistema,
                           prévia, comandos com guarda), journal, files, models — aplicação e rollback
  commands/                hardware, system, fix_engine, dev_laptop (debug) e os comandos legados
src/
  pages/Dashboard.tsx      sistema, sintomas, resultados e "Correções no sistema"
  components/hardware/     SuggestionCard, SystemFixDialog, sintomas
  state/hardware.tsx       identidade do laptop e perfil do sistema
  services/hardware.ts     chamadas da ponte
  design/tokens.css  mascot/   identidade visual e o Pim
```

## Identidade visual e acessibilidade

- Cores em `src/design/tokens.css`, expostas ao Tailwind (`bg-surface`, `text-fg-2`, `text-accent`…); acentos alternativos em Configurações.
- **Pim**, o mascote, é desenhado proceduralmente numa grade 24×20 (`mascot/sprites.ts`) e reage ao que acontece na tela.
- Intensidade de animação **Completa / Suave / Desligada**, respeitando `prefers-reduced-motion`. Foco visível, diálogos com foco preso, `aria-live` nos resultados; status nunca dependem só de cor.
- Atalhos: `Ctrl + K` (páginas e sintomas), `Alt + 1/2` (Início, Configurações), `Esc` (fecha diálogos, exceto durante uma operação).

## Legado (jogos)

O código do FRX para jogos (biblioteca da Steam, Performance Lab com MangoHud, Smart Packs, receitas de jogos, provedores UMU/GitHub) continua no repositório e nos testes, mas não está nas rotas da interface. A documentação dessa fase está em [`docs/`](docs/) (por exemplo [`FIX-ENGINE.md`](docs/FIX-ENGINE.md) e [`DIAGNOSIS.md`](docs/DIAGNOSIS.md)) e no histórico do git.
