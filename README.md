<p align="center">
  <img src="docs/assets/pim.gif" width="144" height="120" alt="Pim waving to welcome you to FRX">
  <br>
  <strong>Hello! Welcome to FRX.</strong>
</p>

# FRX — Play. Fix. Repeat.

[Português (Brasil)](README.pt-BR.md)

**FRX** is a diagnostic, compatibility and troubleshooting center for Windows games on Linux (Steam, Proton and Wine).

**Version 0.5.0:** in the desktop app, FRX detects games installed via Steam, searches for specific **game, GPU, driver and Proton** fixes, measures performance with real logs from **MangoHud**, and **applies fixes** via **Fix Engine**. Recipes reviewed or generated from findings use preview calculated in Rust, verified backups, atomic write per file, disk checking, persistent history, and rollback that preserves third-party changes. The initial class of external patches allows you to download and install UTF-8 configurations with verifiable source, integrity, destination, and requirements.

> The generator recognizes allowed variables and supported configuration files published by recognized sources. Downloads require confirmation and undergo staging and validation before installation. DLLs, executables, scripts, Winetricks, registry, and incomplete procedures remain under review, with reason. No third-party scripts are run and there is no automatic sudo. With Steam open, launch-option-only changes may follow the manual step; file operations wait for it to close. In the browser (`npm run dev:web`), the data and applications are demonstrative.

## Resolution Center — v0.5

Each game's page now brings together **diagnosis, technologies, sources and resolution plans**, using the existing Fix Engine. A fix does not need to increase FPS: compatibility, stability, physics and graphics have their own goals.

- **Diagnose** inspects the installation and environment without changing the game. DLLs are clues; hardware support, configuration, and runtime observation appear separately.
- **Search for solutions** consult documentation from the maintainers and verifiable reports of the same AppID. Cache allows you to continue querying data available offline, with explicit limitations.
- **Import log from this game** analyzes only the chosen file, with up to 2 MiB and corresponding AppID; raw text and personal paths are not persisted in diagnostics.
- Supported plans open the existing preview: **confirm → apply → check → keep or undo**. Recipes are revalidated at the core; external source does not authorize commands.
- **Configuration checked** confirms the status on the disk, not that a crash disappeared or that DLSS/PhysX/FG worked. Performance continues to be measured in the Performance Lab.

There is conservative diagnosis for PhysX, DLSS/DLAA, frame generation, FSR, XeSS, ray tracing, graphics stack, audio/video, controls and stability. The ability to observe each feature varies: MangoHud FPS does not prove frames generated or latency. Proton switching, DLLs and component update mechanisms without specific rollback remain **Requires review**, including documented GE-Proton options. Batman's historical PhysX patch remains unidentified.

Architecture: [Resolution Intelligence](docs/RESOLUTION-INTELLIGENCE.md). Sources and limits: [graphics and physics diagnostics](docs/GRAPHICS-PHYSICS-DIAGNOSTICS.md). Verifiable results: [v0.5](docs/VALIDACAO-v0.5.md) validation.

## Preview

![FRX home screen with library, Fix Packs, FPS comparison and Pim the mascot](docs/assets/frx-home.png)

*Capture of the v0.2 interface in demo mode. Games, hardware and results displayed in this image are fictitious.*

## How to run

FRX is a **Linux desktop application** made with **Tauri 2** (Rust) and an interface in React + TypeScript + Vite.

### Build dependencies

- Node 20.19+ or 22.12+ (tested with Node 24) and npm
- Rust **1.88+** (tested with 1.97) via [rustup](https://rustup.rs); ZIP dependency is `zip` 9.1.1
- System libraries for WebKitGTK:

| Distribution | Command |
| --- | --- |
| Arch/CachyOS/Manjaro | `sudo pacman -S --needed webkit2gtk-4.1 gtk3 base-devel curl wget file openssl librsvg` |
| Debian/Ubuntu | `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev build-essential curl wget file libssl-dev librsvg2-dev` |
| Fedora | `sudo dnf install webkit2gtk4.1-devel gtk3-devel openssl-devel curl wget file librsvg2-devel && sudo dnf group install c-development` |

Optional: `libappindicator-gtk3` (system tray, not yet used) and `fuse2` (to open the AppImage without extracting it; without it, use `APPIMAGE_EXTRACT_AND_RUN=1`).

To **capture** benchmarks, install MangoHud and the game architecture library. On Arch/CachyOS: `sudo pacman -S --needed mangohud lib32-mangohud`. FRX only shows these instructions; it does not install packages. The capture was validated with MangoHud **0.8.4**, Vulkan, NVIDIA RTX 4060 and Hyprland/Wayland. Importing existing logs works even without MangoHud installed.

### Commands

```bash
npm install
npm run dev            # opens the FRX window (Tauri + Vite with live reload)
npm run build          # release binary + AppImage, .deb and .rpm
npm run build:appimage # just the AppImage
npm run build:arch     # native pacman package (Arch/CachyOS)
npm run dev:web        # just the interface in the browser, in http://localhost:1420
npm run build:web      # checks TypeScript and compiles the interface
npm run typecheck
npm test               # front-end tests (Vitest)
(cd src-tauri && cargo test)            # Rust tests (with fixtures, without network)
(cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings)
```

Where are the artifacts:

```
src-tauri/target/release/frx                                   binary
src-tauri/target/release/bundle/appimage/FRX_0.5.0_amd64.AppImage
src-tauri/target/release/bundle/deb/FRX_0.5.0_amd64.deb
src-tauri/target/release/bundle/rpm/FRX-0.5.0-1.x86_64.rpm
packaging/arch/frx-bin-0.5.0-1-x86_64.pkg.tar.zst             (sudo pacman -U …)
```

In the release build, the interface (`dist/`) is **embedded in the binary** and served by the internal Tauri protocol. No need for a Vite server. Vite is only used on `npm run dev`, by `devUrl` `http://localhost:1420`.

### Desktop integration

- Native window with title "FRX", 1440×900, minimum size of 1024×700 and background `#101011` (without white flash when opening).
- The app id and window class are `frx`. `.desktop` files use `StartupWMClass=frx`, so the icon appears right in the taskbar in both Wayland and X11.
- The icon comes from `packaging/icon.svg` (the same d-pad symbol as the interface). To regenerate, run `npx tauri icon packaging/icon.png -o src-tauri/icons`.
- **Tiled window managers** (Hyprland, Sway…) can ignore the minimum size. To open FRX by floating, add a rule for class `frx` in your WM.
- **WebKitGTK + NVIDIA on Wayland:** the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on launch unless you have already set this variable. This avoids the familiar blank window in this combination.

### TypeScript ↔ Rust Bridge

| Command | What is it for |
| --- | --- |
| `scan_installed_games` | Scan Steam (asynchronous) and save the result in cache |
| `get_installed_games` | Last saved scan, to show library instantly |
| `get_system_profile` | Distribution, kernel, session, CPU, RAM, GPUs, driver and version |
| `get_game_art` | Game cover, banner or header, with local cache |
| `search_game_fixes` | Searches for fixes for an AppID across all providers |
| `get_cached_fix_reports` | Saved reports, works offline |
| `get_fix_providers`, `clear_fix_cache`, `get_cache_size` | Source and cache settings |
| `app_info`, `ping` | App information and bridge testing |
| `detect_mangohud` | Executable, version, Vulkan layers and 32/64-bit libraries |
| `import_benchmark`, `list_benchmark_sessions`, `get_benchmark_session` | Import, persistent history and time series |
| `compare_benchmark_sessions`, `update_benchmark_session`, `delete_benchmark_session`, `export_benchmark_session` | Session comparison and management |
| `prepare_benchmark`, `start_capture`, `get_capture_status`, `capture_logging`, `finish_capture`, `cancel_capture`, `reset_capture` | MangoHud Assisted Capture and Control |
| `get_fix_engine_status`, `list_fix_recipes`, `get_fix_recipe`, `check_fix_compatibility` | Local catalog and compatibility (read only) |
| `analyze_fix_findings` | Dynamic generator: category, reason and recipe generated from each finding (does not save in the game) |
| `get_smart_packs`, `compare_smart_packs`, `plan_smart_pack_switch`, `declare_benchmark_pack`, `evaluate_benchmark_session` | Smart Packs: composition, ranking by profile, comparison, step-by-step exchange and declaration of old sessions |
| `prepare_fix_plan`, `apply_fix_plan` | Preview (dry-run) and application of the confirmed plan: only `planId` + `fingerprint` |
| `cancel_fix_operation` | Cancels the preparation/download before the writing phase; does not abandon a transaction while it is writing |
| `list_fix_operations`, `get_fix_operation`, `get_applied_fixes`, `verify_fix_operation` | History, actual status checked on disk and manual steps |
| `prepare_fix_rollback`, `rollback_fix` | Rollback preview and execution |

- **TypeScript:** `src/services/{games,fixes,fixEngine,performance,native,platform,opener}.ts` calls these commands with `@tauri-apps/api/core`. The states are in `src/state/{library,fixes,fixEngine,performance}.tsx`; capture continues when navigating between pages, and Fix Engine progress arrives via event `fix-engine://progress`.
- **Types:** Rust models are mirrored in `src/types/{steam,fixes,fixEngine,performance}.ts`.
- **Permissions:** `capabilities/default.json` releases the Tauri core, native open/save dialogs, and `opener` for the source domains and `steam://rungameid/*`. There is no file system plugin or generic writing API: all writing goes through Fix Engine commands. The asset protocol only sees `$APPCACHE/art/**`, and the CSP remains restricted to the app's files.

## Performance Lab: first benchmark

1. Run `npm install` and `npm run dev` to open the **desktop app**.
2. Open the **Library**, select a game and click **Measure Performance**, or access **Performance Lab** from the sidebar.
3. For an existing CSV, choose **Import benchmark**, select the main MangoHud log (not `_summary.csv`), check the game and conditions, and confirm the import.
4. With the game closed, click **New session**. Choose game, duration, automatic or manual start, HUD and test conditions. Click **Prepare session**. If you change the duration or HUD between sessions, reopen the game to load the new setting.
5. On Steam, open the game's **Properties → General → Launch Options**. Check and paste the line shown by FRX. **Performance Lab does not change these options.** Common variables, quotes, and wrappers such as `gamescope` are preserved. Lines with shell operators or command substitution are rejected instead of rewritten; in this case, use manual import.
6. Click on **Already pasted — wait for game** and open the game. In automatic mode, wait for the configured delay. In manual mode, use **Start Recording Now** or `Left Shift + F2` in game. With the HUD hidden, automatic start depends on connecting to the MangoHud socket.
7. When finished, FRX processes the CSV and saves the session. **Stop and analyze** terminates early via socket; **Discard** stops recording and does not save a session. If the game closes during capture, the available data is analyzed with an interruption warning.
8. In the history, mark two different sessions as **Before** and **After**. Use the same snippet, resolution, preset, and duration for a useful comparison. You can also rename, delete and export sessions.

After capturing, the FRX's own configuration returns to rest, without automatic recording. Remove the FRX line from the Steam options when you no longer want to use MangoHud through this flow.

### Data and methodology

- Settings per game: `~/.local/share/io.frx.FRX/mangohud/<appid>.conf`. MangoHud's global configuration is never changed.
- Capture logs: `~/.local/share/io.frx.FRX/captures/<appid>/`. Persistent history: `~/.local/share/io.frx.FRX/benchmarks/sessions/<id>/`, with `raw.csv`, `meta.json`, and `series.json`. Paths follow `XDG_DATA_HOME` when defined.
- Text CSVs up to **256 MiB**; summary files, binary files, special paths and invalid headers are rejected with an explanatory message.
- Per-frame captures use `log_interval=0`. Average FPS = frames ÷ sum of frametimes. **1% low** and **0.1% low** = inverse of the average of the worst 1%/0.1% of frametimes; require at least 100/1,000 frames. In periodic logs, lows are estimates and appear identified.
- Graphs use reduced series of real data, preserving peaks. Statistics use the original samples. Missing sensors are displayed as “—”.
- Short captures, invalid samples, gaps, interruptions and differences in GPU, resolution, preset or tool generate warnings. An observed improvement does not prove that a Fix Pack caused the difference.
- Assisted captures record the **active FRX corrections** (recipe, version and operation), checked on disk at the end of the capture. The comparison shows when they differ between sessions, without assigning cause. Imported sessions appear as “Not Registered”.
- Capture waits up to 15 minutes to start. Non-growing logs have interrupt handling; previous sessions and `_summary.csv` are never confused with a new capture.
- Control communication uses a Linux abstract socket. Sandboxes without a shared network namespace can impede control; CSV detection and import remain alternatives. The interface provides guidance for the Flatpak variant, but needs to be validated in the user environment.

## Game detection (Steam)

- **Where to look:** known Steam paths — native (`~/.local/share/Steam`, `~/.steam/root`, `~/.steam/steam`, `$XDG_DATA_HOME/Steam`, `~/.steam/debian-installation`), Flatpak (`~/.var/app/com.valvesoftware.Steam/…`) and Snap. Symbolic links are resolved so that equivalent paths do not duplicate the same installation.
- **Other folder:** `FRX_STEAM_ROOT=/caminho` adds a specific root to the search.
- **How to find libraries:** reads `libraryfolders.vdf` (new and old formats) with a real KeyValues ​​parser, without regex. Nothing is scanned recursively.
- **Unavailable libraries:** an unmounted library appears as **unavailable**, and one without permission as **inaccessible**. The games declared in them continue to be listed as "Unavailable".
- **Manifests:** each `appmanifest_*.acf` becomes a game with a status of **Installed**, **Incomplete** (downloading or updating), **Unavailable** (the folder has disappeared) or **Unknown**, depending on the `StateFlags` and the existence of the folder.
- **Tools are not games:** Proton, Steam Linux Runtime, and Steamworks Redistributables are separate from the library.
- **Proton:**
  - the tool **per game** (`config.vdf → CompatToolMapping[appid]`) and the **global default** (`CompatToolMapping["0"]`) are read separately and never confused;
  - the **last run** tool comes from `compatdata/<appid>/config_info`, and the prefix version from `compatdata/<appid>/version`;
  - custom tools come from `compatibilitytools.d` (from user and system, as in `/usr/share/steam/compatibilitytools.d`);
  - when it is not possible to know, the value is `null`; nothing is invented.
- **Playing time:** read from `localconfig.vdf` of the last user who logged into Steam.
- **Hardware:** comes from `/sys/class/drm` (manufacturer, PCI IDs/address, model and chip via `pci.ids`, module/driver version and VRAM when available), `/proc` and `/etc/os-release`. Physical GPUs of the same model remain distinct. Works with any manufacturer, without `nvidia-smi` and without sudo.
- **Covers:** first Steam's own local cache (`appcache/librarycache`), then Steam's public CDN. They are located at `~/.cache/io.frx.FRX/art`. A 404 is remembered for 7 days, and without network the temporary pixel art appears.

### Hardware and software context

The GPU inventory is separate from the GPU used by the game. On multi-card systems, FRX does not turn the interface's probable GPU into a confirmed selection. An assisted capture only associates the MangoHud renderer with a PCI device when there is a unique match; two identical cards may remain indistinguishable. Installed Vulkan drivers also do not prove which driver is active in the game.

Known requirements may restrict manufacturer, PCI device, model, driver version, Proton, and architecture. Missing optional data remains unknown. Community reports and benchmarks only influence the corresponding context: an improvement on the RTX 4060 does not automatically promote the same action on a Radeon or another game. Driver, Proton, and artifact changes reevaluate cache and comparability while preserving history. Details: [Hardware-Specific Fix Intelligence](docs/HARDWARE-FIX-INTELLIGENCE.md).

## Fix discovery

Architecture in `src-tauri/src/fixes/`. Each provider in `providers/` is independent, and adding a source is creating a module and calling it in `fixes::search`.

| Provider | Status | How it works |
| --- | --- | --- |
| **UMU-Protonfixes** | Active | Downloads `gamefixes-steam/<appid>.py` and `gamefixes-umu/umu-<appid>.py` **as text** and resolves symlinks from git. Does a limited static analysis (`pyscan.rs`): docstrings, called `util.*` with literal arguments and `os.environ`. **Never runs or imports the script.** What is not recognized marks the analysis as partial or unverified. |
| **UMU Database** | Active | Public CSV, used to confirm UMU ID and game names from other stores. |
| **GitHub** | Active | Public REST API. Search by **AppID** and perform an additional limited search by name in the integrated trackers; query alone never confirms the game's identity. Reads comments, release notes and metadata from referenced Releases. Keeps the source of the recommendation separate from the source of the file. Uses own User-Agent, ETag, timeouts, limits and temporary blocking by rate limit. `FRX_GITHUB_TOKEN` increases the limit; the token never goes to the front end nor accompanies artifact downloads. |
| **ProtonDB** | **Disabled** | There is no official public API. FRX does not use internal endpoints or scrape the website; it only opens the game page in the browser. |

**Degrees of confidence** (never assigned by popularity):

| Degree | When is it used |
| --- | --- |
| **Verified** | Verifiable maintainer statement: Official release notes citing the game, or a comment from a repository member saying the issue has been fixed. |
| **Curated** | A file from UMU-Protonfixes, a reviewed repository. This does not mean it has been tested on your PC. |
| **Community report** | The same adjustment cited favorably in at least 2 different comments, with more mentions in favor than against. |
| **Not verified / experimental** | Unique, contested mention, or content that could not be interpreted. |

**Compatibility** — each finding is compared to known game, hardware, and tool requirements:

| Status | Meaning |
| --- | --- |
| **Compatible** | The stated requirements correspond to the available data; This does not confirm FPS gains. |
| **Incompatible** | A known requirement contradicts the environment, such as manufacturer, model, driver, or Proton. |
| **Unknown** | Essential data, including the execution GPU in a hybrid system, is missing; the reason remains visible. |
| **Already included** | The configured tool (GE-Proton, proton-cachyos…) already has the same file as UMU-Protonfixes. |

**Grouping:** Community mentions only come together when they mention exactly the same normalized setting (same variable and value, same component or same initialization option). If this adjustment is also in a UMU-Protonfixes script, the mentions become additional sources of it. Every origin remains listed.

**Cache:** is located in `~/.cache/io.frx.FRX/` (library, covers, source responses and reports).
- **Validity:** configurable in Settings (6 hours to 7 days).
- **Update:** "Search again" forces a new query, with revalidation by ETag.
- **No connection:** uses the last saved data and marks it as **outdated**; it is not valid as current confirmation.

## Fix Engine: apply the first fix

In **Fix Packs → Discoveries** or on the game page, **Available Fixes** only contains recipes whose automatic preview has been validated by Rust in the current context. **Apply Fix** opens this preview. Temporary conditions, such as the game or Steam running, show necessary action and allow you to reevaluate. **Findings under investigation** preserves sources, incomplete procedures, and incompatibilities without announcing an available installation. See [Applicability and supported operations](docs/ACTIONABLE-FIXES.md).

1. Open **Fix Packs → Available Fixes** or the game page and choose the installed game.
2. Confirm specific requirements, risks, and impediments. The **Technical Catalog** allows you to consult recipes, but only validation for the chosen game enables automatic application.
3. **Apply fix** opens the preview: launch options, affected files, download source/version if applicable, requirements, risks and backups. Nothing has been written yet.
4. Confirm. With Steam and the game closed, Pim shows the actual steps: download/verify when necessary, create backups, save and verify. If the source, environment, or files change after listing, please re-evaluate to get a current preview. FRX does not close programs.
5. In **History**, each operation shows steps, before/after lines, backup (with SHA-256) and the **Undo** button. Rollback only removes what the fix added and refuses to overwrite changes made by other programs.

Details, recipe format, allowlist and states: [`docs/FIX-ENGINE.md`](docs/FIX-ENGINE.md). Validation of this version: [`docs/VALIDACAO-v0.4.0.md`](docs/VALIDACAO-v0.4.0.md).

### Fixes that require external files

There is also automatic generation of adjustments **typed from `dxvk.conf`** when the source assigned to the game specifies the relative path, the allowed key and its value. FRX creates or merges the configuration while preserving unrelated options. Variables, DXVK configuration, and verifiable artifacts can be part of a single transaction; an unknown step prevents partial application. There is no assumed performance gain and no confirmation that the game read the configuration just because it was written.

FRX identifies assets from GitHub Releases linked by integrated sources. The initial application accepts **INI, CFG, CONF, JSON and TXT in UTF-8**, either directly or as explicitly selected members of a ZIP. The publisher must provide linkage to the AppID, relative targets, and sufficient digests; replacements also require the hash of the expected original. Strict instructions or a typed release manifest can generate a recipe without registering JSON per game in FRX. Ambiguous prose or procedures with unknown steps are subject to review.

After preview and confirmation, the same Fix Engine downloads to private staging, revalidates the current metadata, checks SHA-256 and content, creates backups and installs only authorized files. ZIPs are never extracted directly upon installation. History allows you to undo after restarting the FRX; External changes or a Steam update may require revision rather than overwriting. There is no automatic reapplication after updating.

Recognized origin and checksum do not guarantee that a file is benign or faster. DLLs, executables, scripts, TAR, XML and procedures that change prefixes/Proton **do not have automated installation at this stage**. Batman's historical NVIDIA/PhysX patch has not yet been safely identified. See [Automatic File Fix Engine](docs/AUTOMATIC-FILE-FIX-ENGINE.md) and the [Batman/PhysX](docs/BATMAN-PHYSX-INVESTIGATION.md) investigation.

## Smart Packs

Discoveries, recipes, packages, and evidence maintain their link to the **Steam AppID and the hardware/software environment**. An improvement or regression in another game, GPU or incompatible driver does not change the current ranking. General technical documentation explains risks; only pertinent evidence and comparable local measurements support contextual assessment. See [Game-Specific Fix Intelligence](docs/GAME-SPECIFIC-FIX-INTELLIGENCE.md) and [Hardware-Specific Fix Intelligence](docs/HARDWARE-FIX-INTELLIGENCE.md).

In **Fix Packs → Smart Packs** (or from the summary on the game page), FRX assembles combinations of fixes applicable to the game, detects conflicts and redundancies and classifies the packages according to what was **measured** in the Performance Lab. The classification changes depending on the profile: Maximum performance, Balanced or Maximum compatibility.

- FRX automatically separates alternatives with **risk of loss of performance**, such as WineD3D and disabling synchronizations. The warning appears before any test; it is not a universal regression. Reliable reporting of the selected AppID can justify a candidate while preserving the notice.
- Without sufficient benchmarks (≥ 2 sessions comparable with the package and baseline), there is no measured winner. FRX does not turn compatibility reports into FPS promises and leverages existing measurements when available.
- Packages can combine allowed variables and supported files. They are applied by the Fix Engine as **one** operation, with backups per file, journal and rollback. An artifact without validation prevents the application of the entire package; atomic writing of each file does not imply atomicity of the entire transaction.
- The identity of installed files includes destination, hash, and artifact version/source. Updating a patch does not silently reuse evidence from another configuration.
- Switching packages and restoring the previous state are done step by step, each step with its own preview.
- Each assisted capture records the active pack. Old sessions can be declared.
- **Test performance** guides reference → apply → capture and automatically compares the capture with the reference: improvement, regression, no difference or inconclusive, marked as *preliminary* while there are few sessions. With regression, it offers to restore the previous configuration.

Details: [`docs/SMART-PACKS.md`](docs/SMART-PACKS.md). Validation: [`docs/VALIDACAO-SMART-PACKS.md`](docs/VALIDACAO-SMART-PACKS.md).

## Security

- **Discovery and Performance Lab:** sources are read as text/metadata; they do not run third-party scripts or install DLLs. Prefixes and drivers are not changed, and there is no automatic sudo. The detection only runs `mangohud --version`, with explicit arguments and timeout. The game is only opened by Steam when the user requests it.
- **Fix Engine:** writes only the plan confirmed and revalidated by the core: `LaunchOptions` game options and supported files within the resolved installation. Downloads require recognized source, identified release/asset, SHA-256, compatible requirements and documented destination; content is checked before installation. Variables go through allowlist; actions are typed, no generic shell. Paths refuse escapes, links and targets changed since the preview. Backups and journals precede writing; each file uses atomic replacement, but a multi-file transaction may require recovery after failure. Rollback detects discrepancies and does not declare partial restoration as complete. FRX never terminates Steam or processes. Backups and journal are in `~/.local/share/io.frx.FRX/fix-engine/`, with directories 0700.
- All internet content is treated as untrustworthy:
  - the HTTP client only accepts HTTPS for a closed list of hosts, including in redirects, and limits the size of responses;
  - malformed JSON is discarded;
  - only narrow patterns are extracted from comments, and lines with `;`, `|`, `&&` or `$(` are discarded;
  - external texts are displayed as text, never as HTML.
- External links are validated twice: in TypeScript and in the `opener` capability.

### Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl + K` | Quick search (pages, games, Fix Packs) |
| `Alt + 1…5` | Go to Home, Library, Fix Packs, Performance Lab, Settings |
| `/` | Focus the Library search |
| `Esc` | Close dialogs (except during an operation in progress) |

## Stack

Tauri 2 (Rust) · React 19 · TypeScript · Vite · Tailwind CSS 4 · Motion · Recharts · Lucide. The fonts (Bricolage Grotesque, Geist, Geist Mono and Silkscreen) come from Fontsource and are packaged locally, without depending on the network.

## Structure

```
src-tauri/src/
  commands/                Tauri commands: app, games, system, fixes, fix_engine, performance
  fix_engine/              catalog, security (allowlist), planner, files, confined (FD-based destinations),
                           artifacts (download/staging), journal, models — application and rollback
  performance/             parser, metrics, comparison, persistence, detection, capture, models
  steam/                   vdf (parser), discovery, libraries, manifests, compatibility, userdata,
                           launch_options (shared tokenizer), localconfig (positional editing),
                           processes (Steam/game running, read-only)
src-tauri/recipes/         local recipe catalog (JSON + SHA-256 manifest)
  fixes/                   models, http (cache + rate limit), pyscan, tweaks, aggregation, compat
    providers/             umu (protonfixes + database), github, artifacts, protondb (disabled)
  system/                  profile and GPUs via /proc and /sys, context (requirements and environment)
  cache/  art.rs  net.rs   file cache, cover art, restricted HTTP client
packaging/               source icon (SVG/PNG) and Arch package (PKGBUILD + .desktop)
docs/assets/             interface screenshot and Pim animation for this README
src/
  app/routes.ts            main navigation
  layouts/                 AppShell, Sidebar, CommandPalette, Page
  pages/                   Home/RealHome, Library, GameDetail/SteamGameDetail, FixPacks/FindingsPage,
                           Performance/RealPerformance, Settings, NotFound
  components/
    ui/                    Button, controls, Dialog, Toasts, status, counters, empty states
    game/                  GameArt, CoverImage, GameCard/GameRow, installation status and warnings
    fix/                   FindingCard, RecipesPanel, ApplyFixDialog, OperationHistory,
                           FixFlowDialog (demo), ChangeList, sources
    perf/                  CompareHero, MetricChart, verdicts
    performance/           real capture, import, history, charts and comparison
    brand/                 Logo (d-pad symbol + wordmark)
  design/tokens.css        design tokens (colors, radii, motion, accents)
  mascot/                  Pim: procedural sprites, shared clock, component
  data/                    demo data (games, problems, fix packs, benchmarks…)
  services/                games, fixes, performance, native (invoke), opener, platform and demo.ts
  state/                   store, library (real Steam × demo), fixes, fixEngine, performance, motion, selectors
  hooks/                   useGameArt
  test/                    fixtures and IPC bridge integration tests (Vitest + mockIPC)
  types/                   domain types
```

## Visual identity

- **Tokens:** all colors live in `src/design/tokens.css` and are exposed to Tailwind via `@theme inline` (`bg-surface`, `text-fg-2`, `text-accent`…). Alternative accents (`menta`, `coral`, `ceu`) already work in Settings; New themes just need to reset the variables under `[data-theme='…']`.
- **Symbol:** a directional pad (d-pad) in pixel art that is also a first aid "+": play and repair in the same gesture.
- **Pim:** the original mascot, a marshmallow with a butter-yellow hat. It is drawn procedurally on a 24x20 grid (`mascot/sprites.ts`) and rendered as a crisp SVG (one `<path>` per color). It has 13 poses, including wave, rest, investigate, repair, observe, celebrate, prepare, record, and analyze benchmarks.
- **Covers:** each fictional game receives a pixelated scene generated in SVG from a fixed seed (`GameArt.tsx`), without third-party assets.

## Movement and accessibility

- A single 8Hz clock animates all sprites and pauses when the window is hidden or when no Pim is visible.
- Intensity **Full / Soft / Off**, plus "Follow system / Always reduce" for `prefers-reduced-motion`. The effective level goes to `<html data-motion>` and `MotionConfig`.
- Visible focus on all controls, "Skip to content" link, segmented controls navigable by arrows, dialogs with focus trapped, `aria-live` in progress.
- Compatibility status, risk and severity never depend solely on color: they always come with a pixelated glyph and text.
- In the charts, "before" is dashed and "after" is solid, with legend and tooltip.

## What is real and what is still simulated

| Area | In desktop app (v0.4) | In browser |
| --- | --- | --- |
| Library | **Real:** Steam games detected by Rust | 12 fictional games |
| System | **Actual:** /proc, /sys, os-release | Fictitious profile |
| Covers | **Real:** Steam cache or public CDN, with local cache | Generated pixel art |
| Fixes | **Actual discovery** by AppID/hardware; **generated recipes** for recognized actions, including UTF-8 configurations from verifiable Releases; application by Fix Engine, with backup and rollback | Dummy Fix Packs with simulated apply and revert |
| Performance Lab | **Actual:** MangoHud logs, assisted capture, history and comparison | Demo |
| Persistence | Fix Engine local cache, benchmarks, journal, and backups in data directory, and preferences | Preferences only (localStorage) |

In Settings → Data and sources, the "Use demo data" option switches the desktop app to the demo, useful for testing.

The app uses `HashRouter` and relative paths (`base: './'`) to work bundled.
