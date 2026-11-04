# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

Hardware Fixer is the hardware-focused version of FRX. It replaces the game
screens with a laptop dashboard. The Rust engine (Fix Engine, provider, cache
and system detection) was kept and narrowed to hardware.

### Added

- Laptop identification from DMI (vendor, model, board). Generic firmware strings
  count as missing, and on Lenovo the commercial name comes from `product_version`.
- Arch Wiki search for each symptom (Wi-Fi, audio, touchpad, suspend, firmware),
  with a strict model match ("Gen 9" never matches "Gen 10").
- Parser for unambiguous instructions: `options <module> <key=value>` and kernel
  parameters. Instructions with conditions are flagged and need an explicit
  acknowledgement.
- System fixes with a preview of the exact file, backup, checks and the command
  to run. Supported targets: modprobe options (`/etc/modprobe.d/frx-*.conf`),
  kernel parameters in `/etc/default/grub`, and a fixed sleep-service template.
- Verification of the file hash in `/etc` after applying or undoing a fix.
- Undo for applied fixes, and cancelling a pending fix from its dialog.
- Guard against overwriting a file that changed after the preview (root-side
  `sha256sum --check`).
- Kernel parameters that need GRUB are flagged on the card when GRUB is absent.

### Changed

- The app is branded Hardware Fixer. The window title and interface use the new
  name; the binary and package stay `frx`.

### Removed

- Game code: library and game screens, Performance Lab (benchmarks, MangoHud),
  Smart Packs and fix packs, the Steam integration, game recipes, the game
  diagnosis pipeline and the game fix engine.
- Game-only providers (UMU, GitHub, ProtonDB, Reddit, WineHQ, Steam guides,
  artifacts) and the fwupd skeleton, which was never enabled.
- Local AI query (llama.cpp and its model handling) and the `zip` dependency.
- The file-dialog plugin, the asset protocol and the unused `recharts` package.
- The demo data mode, the `ping` test command and the preferences that no longer
  had a screen or an effect (`scanOnStartup`, `confirmBeforeApply`,
  `backupRetentionDays`, `tempUnit`, `useDemoData`).

### Security

- Closed write allowlist, with `O_NOFOLLOW` on every directory. Symlinks, hard
  links and special files are refused.
- No automatic `sudo`, `pkexec` or `sed -i`. Protected boot parameters (`init=`,
  `rd.break`, `mitigations=`, `selinux=`) are never applied.
- HTTPS only to a closed list of hosts, with bounded response size.
