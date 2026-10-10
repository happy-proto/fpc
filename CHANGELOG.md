# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Interactive fuzzy selection of files and directories, including hidden and ignored paths by default.
- Copy an absolute path directly from a positional argument.
- Optional ignore rules and explicit ignore files.
- Native clipboard, SSH OSC 52, and tmux clipboard support, with path output as a fallback.
- Dynamic shell completion with explicit installation into standalone files for fish, bash, and zsh, without editing shell startup files; Bash installation checks for bash-completion first.
- Path-first Tab completion without a standalone current-directory entry, with option suggestions after `--` and typed option values such as shell names.
- Isolated shell completion E2E checks on Linux and macOS, including real Tab presses and quoted paths, with a required CI gate.
- English and Chinese README, contribution and release conventions, and a Keep a Changelog change log.
- mise development tasks, mbx-backed macOS/Linux CI, and Cargo/GitHub Actions dependency updates.
- A one-time `mise run setup` development initialization entry point, currently installing optional hk pre-commit hooks for whitespace, Rust formatting, and fast Rust tests.
- Fast PTY checks for clipboard output, picker selection and cancellation, ignore rules, and terminal restoration, without desktop or shell completion dependencies.
- Native Ghostty acceptance checks using real SSH, tmux, and system clipboards on Linux and macOS.
- Versioned four-platform release archives, checksums, cargo-binstall-compatible packaging, and combined maintained and generated release notes.
- A rolling nightly prerelease rebuilt from successful main commits, with refreshed publication dates, exact revision information, and maintained Unreleased notes.
- Detailed version output with source commit, dirty state, build time, target, compiler, and repository link.
- A release runbook covering version preparation, publication, installation verification, and partial failures.

### Changed

- Copy operations no longer print success messages; errors, clipboard fallback warnings, and explicit `--print` output remain visible.
- Completion installation prints its destination on a separate line for easy selection and copying.
- Absolute paths preserve symbolic links instead of resolving their targets, including dangling links.

[Unreleased]: https://github.com/happy-proto/fpc/commits/main/
