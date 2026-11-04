# Security policy

Hardware Fixer can write files under `/etc` (through a root-side step that the
user runs). Security reports are taken seriously.

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for a security problem.

Use GitHub's private reporting instead: open the repository's **Security** tab
and choose **Report a vulnerability**. Include:

- the Hardware Fixer version (the release tag or the package version);
- your distribution and kernel;
- the steps to reproduce, and what you expected to happen.

This is maintained by one person, so responses are best-effort. Fixes are
described in the release notes, and the reporter is credited if they want that.

## What is in scope

- Writes outside the allowlist in `src-tauri/src/fix_engine/confined.rs`
  (`/etc/modprobe.d/frx-*.conf`, `/etc/systemd/system/frx-*.service` and
  `/etc/default/grub`), including symlink, hard link or race tricks.
- Any path that runs a privileged command without the user running it
  themselves, or that skips the hash check before overwriting a file.
- Content from the Arch Wiki or the network being treated as code or HTML.
- Network requests to hosts outside the closed list.

Problems in Arch Linux, the Linux kernel, the wiki itself or other software are
out of scope; please report them upstream.
