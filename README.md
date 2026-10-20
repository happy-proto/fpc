# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)
[![Dependencies](https://deps.rs/repo/github/happy-proto/fpc/status.svg)](https://deps.rs/repo/github/happy-proto/fpc)

[简体中文](README.zh-CN.md)

Find a file or directory and copy its absolute path, with a short command.

When pointing an Agent at a file, paths outside the workspace or hidden by `.gitignore` can be awkward to select. `fpc` makes them easy to find with a built-in fuzzy picker, locally or over SSH and tmux.

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
fpc --print ./src/main.rs    # Print the path instead of copying
fpc --install-completion     # Auto-detect your shell and install completion
fpc --help                   # Explore all options
```

Hidden files and ignored paths are included by default.

Path Tab completion prefers case-insensitive prefixes and falls back to nucleo fuzzy matching when no prefixes match, so `readmemd<Tab>` and `rdm<Tab>` can find `README.md`. Completion searches one directory level at a time and requires a leading `.` to show hidden names. Tab only inserts a path; it does not change the clipboard.

In Fish, multiple path matches open a compact menu below the command line, showing up to five candidates at a time without entering the alternate screen. Use ↑/↓ or Tab/Shift+Tab to select and Enter to insert. Esc or Ctrl+C cancels and preserves your original input; to edit the query, cancel and continue typing in the shell. Resizing the terminal dismisses the menu. A unique match is inserted immediately. The menu uses fpc's full candidate list and ranking without Fish filtering or changing the query's casing. Options and shell names retain ordinary completion. `--install-completion fish` installs a managed `conf.d/fpc.fish` snippet to load the Tab adapter at startup; other commands keep their existing Tab behavior.

Bash and Zsh use their native completion menus; the shell controls the final presentation of fpc's ranked candidates.

The picker highlights the characters that matched your query, even when they are spread across the path, and scrolls long results to reveal the matching region. Type `'dcjanus` (a single leading quote) to require the continuous substring `dcjanus`; plain `dcjanus` uses fuzzy matching.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributions, [RELEASE.md](RELEASE.md) for the release procedure, and [CHANGELOG.md](CHANGELOG.md) for changes.
