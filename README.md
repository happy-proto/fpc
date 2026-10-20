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

Tab completion matches file and directory names by case-insensitive prefix, falling back to case-insensitive contiguous substrings only when there are no prefix matches. For example, `readme<Tab>` matches `README.md`, and `zh<Tab>` can match `README.zh-CN.md`. It completes one directory level at a time and requires a leading `.` to offer hidden names. Nonconsecutive matching is reserved for the picker.

The picker highlights the characters that matched your query, even when they are spread across the path, and scrolls long results to reveal the matching region. Type `'dcjanus` (a single leading quote) to require the continuous substring `dcjanus`; plain `dcjanus` uses fuzzy matching.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributions, [RELEASE.md](RELEASE.md) for the release procedure, and [CHANGELOG.md](CHANGELOG.md) for changes.
