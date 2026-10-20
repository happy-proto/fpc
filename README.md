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

Path Tab completion uses your shell's ordinary completion menu. It prefers case-insensitive prefixes and falls back to nucleo fuzzy matching when no prefixes match, so `readmemd<Tab>` and `rdm<Tab>` can find `README.md`. fpc returns candidates in score order; the shell controls their final presentation and may insert a unique match immediately. Completion searches one directory level at a time and requires a leading `.` to show hidden names. Options and shell names also retain ordinary completion. Tab does not open fpc's picker or change the clipboard.

Fish uses a small Tab adapter to align the query's casing with the top candidate before invoking your existing Tab binding. This lets its native, case-sensitive subsequence filter accept queries such as `readmemd` for `README.md`. Fish still applies its own match-quality rules, so its visible results can differ from fpc's full candidate list. `--install-completion fish` installs a managed `conf.d/fpc.fish` snippet to load the adapter at startup; other commands keep their existing Tab behavior.

The picker highlights the characters that matched your query, even when they are spread across the path, and scrolls long results to reveal the matching region. Type `'dcjanus` (a single leading quote) to require the continuous substring `dcjanus`; plain `dcjanus` uses fuzzy matching.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributions, [RELEASE.md](RELEASE.md) for the release procedure, and [CHANGELOG.md](CHANGELOG.md) for changes.
