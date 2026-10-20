# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)
[![Dependencies](https://deps.rs/repo/github/happy-proto/fpc/status.svg)](https://deps.rs/repo/github/happy-proto/fpc)

[简体中文](README.zh-CN.md)

`fpc` finds files and directories and copies their absolute paths to the clipboard.

During AI coding, you often need to pass an absolute file path to an Agent. Files outside its workspace or hidden by `.gitignore` can be awkward to select in an editor. `fpc` provides a fuzzy picker in the terminal, or you can pass a path directly.

When run over SSH, `fpc` copies the remote file's absolute path to your local clipboard. It also works inside tmux.

## Install

Install the latest stable release with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall fpc
```

## Usage

```sh
fpc                    # Find a file or directory and copy its absolute path
fpc ./src/main.rs      # Copy the specified file's absolute path
```

The picker searches beneath the current directory, including hidden and Git-ignored paths.

Use `fpc --install-completion` to enable Tab completion for Bash, Fish, or Zsh. Run `fpc --help` for all options.

[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [MIT License](LICENSE)
