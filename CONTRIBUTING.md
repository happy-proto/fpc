# Contributing

Use English for code, CLI text, documentation, and pull requests. Keep `README.md` and `README.zh-CN.md` aligned when user-facing behavior changes.

Work on a separate branch. Experimental commits are welcome; pull requests are squash-merged into `main`. Use an English Conventional Commit title for the PR, and describe the resulting behavior and any migration needs in its body: these become the squash commit message.

The repository uses the stable Rust toolchain selected by [rust-toolchain.toml](rust-toolchain.toml). Install the pinned development tools with `mise install`, then run the standard checks before opening a pull request:

```sh
mise run check
mise run test
```

Python test and release scripts are executable single-file CLIs managed by uv. They declare Python 3.15+ and their dependencies inline; uv obtains the interpreter and packages automatically, so no separate Python installation is required. Run the script directly or use `uv run --script <script>` where direct execution is unavailable. CI uses the same uv version configured in `mise.toml`.

`mise run check` runs formatting and Clippy. `mise run test` runs the Rust tests, fast PTY behavior tests, and release validation tests. These commands use the same test selection on every machine and do not require a GUI, SSH server, shell completion framework, or access to your clipboard. Use `mise run build` to build the release binary and `mise run install` to install it from this checkout. `mise run ci` runs the local checks and fast tests together.

Test public behavior, edge cases, and regressions. When fixing a bug, first reproduce it with a failing test when practical. Clipboard tests should use isolated command stubs or terminal fixtures, rather than overwrite a contributor's real clipboard.

Prefer CLI and shell integration tests over tests tied to internal helpers. CI installs Bash 4.2+, Fish, Zsh, and bash-completion, then always runs the complete shell suite on Linux and macOS. It uses temporary HOME/XDG directories and real Tab presses, executes `fpc --print`, and leaves the system clipboard untouched. Missing prerequisites fail the suite; no test is automatically skipped based on the host environment. `mise run e2e` is an explicit full-shell entry point for an environment prepared like CI; it is not part of the default local checks. `BASH_COMPLETION_FILE` can select the framework's main script explicitly.

The Terminal acceptance workflow uses real Ghostty, OpenSSH, tmux, and the system clipboard on both operating systems. This is separate from the fast tests. `mise run acceptance` runs it explicitly in a prepared desktop environment and writes to that environment's clipboard; it is not part of local checks or hooks. CI keeps diagnostic artifacts when it fails.

Record notable user-facing changes in the appropriate category under `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Do not copy commit logs into it. Follow [RELEASE.md](RELEASE.md) when preparing and publishing a version.

After `mise install`, run `mise run setup` once to initialize the local development environment. It currently installs repository-local Git hooks managed by hk; future development setup steps belong here. Run `mise run hooks-check` to check without installing hooks. Pre-commit checks whitespace, Rust formatting, and the fast test suite when relevant files change. They check the working tree without fixing, staging, or stashing files. Complete shell and desktop acceptance tests remain outside hooks.
