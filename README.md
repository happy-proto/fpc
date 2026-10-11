# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![Dependencies](https://deps.rs/repo/github/happy-proto/fpc/status.svg)](https://deps.rs/repo/github/happy-proto/fpc)

[简体中文](README.zh-CN.md)

Find a file or directory and copy its absolute path, with a short command.

When pointing an Agent at a file, paths outside the workspace or hidden by `.gitignore` can be awkward to select. `fpc` makes them easy to find, locally or over SSH and tmux. Written in Rust, with a built-in fuzzy picker.

## Install

Install the latest stable release with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall fpc
```

## Usage

```sh
fpc                          # Fuzzy-pick a file or directory below the current directory
fpc ./src/main.rs            # Copy a path directly
fpc --respect-ignore        # Opt in to .gitignore and .ignore rules
fpc --ignore-file ./rules    # Apply a custom Git-style ignore file
fpc --print ./src/main.rs    # Print the path instead of copying
fpc --install-completion fish # Install dynamic Tab completion (also bash and zsh)
```

Hidden files and ignored paths are included by default. Hidden paths rank slightly lower unless explicitly requested, while close matches remain easy to find. Tab completion normally offers files and directories within the path being typed, without a standalone `.` entry; use the picker for recursive searches. Type `--` to complete options. Option values complete according to their type, including shell names after `--install-completion`.

In the picker, type to search, use arrow keys or Tab to select, Enter to copy, and Esc to cancel. Paths become absolute without resolving symlinks; a link keeps its own path, even if its target is missing.

Completion installation is explicit and writes a separate file without editing shell startup files. Fish loads it automatically; Bash requires installed and enabled `bash-completion` (installation fails if it cannot be detected). For Zsh, add `${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions` to `fpath` before `compinit`. The installer prints the location; upgrades usually do not require reinstalling completion.

Local sessions use the system clipboard; SSH uses terminal clipboard support (OSC 52), and tmux uses its clipboard channel. Copying is silent on success. Remote copying requires a compatible terminal. If no clipboard channel is available, a warning is shown and the path is printed for manual copying.

## Build locally

```sh
cargo install --path . --locked
```

`fpc --version` shows the source commit and build information; `fpc -V` prints only the package version. Uncommitted source changes are marked `dirty`.

The [nightly prerelease](https://github.com/happy-proto/fpc/releases/tag/nightly) tracks successful builds of `main`. Stable releases are published only from matching version tags.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributions, [RELEASE.md](RELEASE.md) for the release procedure, and [CHANGELOG.md](CHANGELOG.md) for changes.
