# Contributing

Install Rust through rustup and the development tools through [mise](https://mise.jdx.dev/):

```sh
mise install
mise run check
mise run test
```

Checks cover formatting, Clippy, workflows, and fast tests without changing your clipboard or requiring shell completion frameworks. Python scripts use uv, which downloads Python 3.15+ and their dependencies automatically.

CI uses Cargo with sccache for compiler caching and disables incremental compilation only on runners. Local development continues to use mbx. Cargo registry and Git downloads are cached separately; only the native checks save download caches on `main` pushes. Compiler cache writes are also restricted to `main` pushes; PRs, merge queues, manual runs, and stable tags read existing entries. Build jobs attach sccache JSON statistics and Cargo timings as short-lived artifacts, with cache statistics in the job summary. Use these reports to compare cold and warm builds, including cache misses, non-cacheable requests, and storage errors.

Workflows use compatible major Action tags where upstream provides them; `0.0.x` Actions and Actions without a compatible moving tag retain exact versions. The native `.github/workflows/actions.lock` pins the implementations, including dependencies of the shared actions in `.github/actions/`, referenced with the native `$/` syntax. Install the official verifier with `gh extension install github/gh-actions-lock --pin v0.2.0` before running checks. After editing Action references, run `gh actions-lock --no-narrow` and review the full diff. `mise run check-workflows` lints workflows and verifies the lockfile online. During weekly dependency maintenance, review moving tags and refresh their pins with `gh actions-lock --relock --no-narrow`; ordinary generation preserves existing pins.

PRs run checks, terminal acceptance, and release validation independently. On `main` and stable-tag pushes, Release calls the reusable CI workflow and waits for both its checks and release validation before publishing. The existing required check names remain the PR merge gates.

Use `mise run install` to try your changes locally. `mise run setup` optionally installs Git hooks. CI also tests Bash, Fish, Zsh, and real clipboard behavior on Linux and macOS.

Open a PR from a separate branch with an English Conventional Commit title. Describe the change and test relevant behavior. PRs are squash-merged into `main`.

Use English for code, CLI text, and documentation; keep the Chinese README aligned. Record notable changes under `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Maintainers can follow [RELEASE.md](RELEASE.md) to publish a version.
