# Contributing

Use English for code, CLI text, documentation, and pull requests. Keep `README.md` and `README.zh-CN.md` aligned when user-facing behavior changes.

Work on a separate branch. Experimental commits are welcome; pull requests are squash-merged into `main`. Use an English Conventional Commit title for the PR, and describe the resulting behavior and any migration needs in its body: these become the squash commit message.

The repository uses the stable Rust toolchain selected by [rust-toolchain.toml](rust-toolchain.toml). Install the pinned development tools with `mise install`, then run the standard checks before opening a pull request:

```sh
mise run check
mise run test
mise run e2e
```

`mise run check` runs `cargo fmt --check` and `mbx clippy --all-targets --locked -- -D warnings`; `mise run test` runs `mbx test --locked`. Use `mise run build` to build the release binary and `mise run install` to install it from this checkout. `mise run ci` runs checks, tests, and shell E2E together.

Test public behavior, edge cases, and regressions. When fixing a bug, first reproduce it with a failing test when practical. Clipboard tests should use isolated command stubs or terminal fixtures, rather than overwrite a contributor's real clipboard.

Prefer CLI and shell integration tests over tests tied to internal helpers. `mise run e2e` requires Bash 4.2+, Fish, Zsh, bash-completion, and uv. It installs completion in temporary HOME/XDG directories, uses Fish's public `complete -C` interface for candidate checks, and sends real Tab keys through a pseudo-terminal to verify loading, navigation, and quoting. It executes `fpc --print` and leaves your clipboard and shell configuration untouched. Set `BASH_COMPLETION_FILE` to the framework's main script for a custom installation. CI runs this on Linux and macOS; missing prerequisites fail the check.

Record notable user-facing changes in the appropriate category under `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Do not copy commit logs into it. See [MAINTAINING.md](MAINTAINING.md) for release policy.

After `mise install`, run `mise run setup` once to initialize the local development environment. It currently installs repository-local Git hooks managed by hk; future development setup steps belong here. Run `mise run hooks-check` to check without installing hooks. Pre-commit checks whitespace, Rust formatting, and the Rust test suite when relevant files change. They check the working tree without fixing, staging, or stashing files. Cross-shell E2E remains in CI and `mise run e2e`.
