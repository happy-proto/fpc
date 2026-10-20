# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)

[简体中文](README.zh-CN.md)

Copy a file path without leaving the terminal.

When you need to point an AI assistant or a teammate at a file, run `fpc`, type a few characters, and press Enter. Its absolute path goes straight to your clipboard.

```sh
fpc                  # Find a file or directory beneath the current directory
fpc src/main.rs      # Copy a known path directly
```

Hidden files and Git-ignored paths are included, so `.env` and build outputs are easy to find. The same command works locally, over SSH, and in tmux.

## Install

Install the latest stable release with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall fpc
```

## Shell completion

Enable Tab completion for Bash, Fish, or Zsh:

```sh
fpc --install-completion
```

Run `fpc --help` for more options, including printing paths instead of copying them.

[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [MIT License](LICENSE)
