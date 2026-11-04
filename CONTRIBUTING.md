# Contributing to Hardware Fixer

Thanks for helping. The most useful contributions right now are:

- **Laptop reports:** does the model get identified? Did a suggestion work?
  Open an issue with the "Laptop report" template.
- **Bug fixes**, with a test that fails before the fix.
- **Parser and provider improvements** for the Arch Wiki, with fixtures.

## Setup

Requirements are listed in the [README](README.md#build-dependencies): Node
20.19+ or 22.12+, Rust 1.88+ and the WebKitGTK libraries.

```bash
npm install
npm run dev          # desktop app with live reload
```

## Checks

Run these before opening a pull request. CI runs the same commands.

```bash
npm run typecheck
npm test
(cd src-tauri && cargo clippy --all-targets -- -D warnings)
(cd src-tauri && cargo test)
```

The live tests (`cargo test -- --ignored`) reach the Arch Wiki and read the real
`/etc`. Run them on purpose, not in CI.

## Rules that matter for this project

- **FRX never runs `sudo`, `pkexec` or `sed -i`.** The app writes a staged file,
  shows the command and lets the user run it. Keep it that way.
- **Writes have an allowlist.** A new write target needs a change in
  `src-tauri/src/fix_engine/confined.rs`, tests for symlinks and for a file that
  changed after the preview, and a note in the README table.
- **Unambiguous instructions only.** The parser must reject negative sentences,
  commands and unit names. When in doubt, flag the instruction instead of
  applying it.
- **Keep the UI in Portuguese (pt-BR)** and the docs in both `README.md` and
  `README.pt-BR.md`.

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(scope):`,
  `fix(scope):`, `docs:`, `chore:`, `test:`.
- Keep a pull request to one change. Describe what you tested and on which
  hardware, if it touches detection or fixes.
- Update `CHANGELOG.md` under *Unreleased*.
