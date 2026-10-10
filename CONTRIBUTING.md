# Contributing

Install Rust through rustup and the development tools through [mise](https://mise.jdx.dev/):

```sh
mise install
mise run check
mise run test
```

Checks cover formatting, Clippy, and fast tests without changing your clipboard or requiring shell completion frameworks. Python scripts use uv, which downloads Python 3.15+ and their dependencies automatically.

Use `mise run install` to try your changes locally. `mise run setup` optionally installs Git hooks. CI also tests Bash, Fish, Zsh, and real clipboard behavior on Linux and macOS.

Open a PR from a separate branch with an English Conventional Commit title. Describe the change and test relevant behavior. PRs are squash-merged into `main`.

Use English for code, CLI text, and documentation; keep the Chinese README aligned. Record notable changes under `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Maintainers can follow [RELEASE.md](RELEASE.md) to publish a version.
