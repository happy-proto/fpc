# Maintaining fpc

## Repository and merges

The repository, crate, and binary are all named `fpc`. Keep `main` as the default branch and perform development on separate branches.

Only squash merge is enabled. The squash commit uses the PR title and body; source branches are deleted after merging. Do not publish experimental branch commits as release history.

Require the CI status check named `required` in branch protection once the workflow has been published and run. This check includes formatting, Clippy, CLI tests, and shell completion E2E on Linux and macOS; failed, cancelled, or skipped checks must block merging.

## Versions and changelog

Use [Semantic Versioning](https://semver.org/spec/v2.0.0.html), starting at `0.0.1`. Each published version has its own `v<version>` tag and GitHub release. Never overwrite a published tag or release; do not maintain a rolling `latest` release.

Follow [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) in `CHANGELOG.md`:

- Maintain an `Unreleased` section during development.
- Group notable changes under Added, Changed, Deprecated, Removed, Fixed, or Security; omit empty categories.
- At release time, move the entries into a version section with the actual release date (`YYYY-MM-DD`), newest first. Leave an `Unreleased` section for subsequent work.
- Use release or comparison links for version headings. Once the first version is tagged, make the `Unreleased` link compare that tag against `HEAD`.
- Describe user-visible behavior and migration requirements. Do not generate this file from commit subjects.

## Release notes

Each GitHub release includes two sections:

1. **Highlights**: the maintained notes from that version's `CHANGELOG.md` section, preserving its categories.
2. **GitHub-generated changes**: GitHub's automatic release notes, including the comparison link and contributor information when available.

Treat the versioned changelog as the source of truth for the maintained notes. Derive the release section from it rather than maintain a separate duplicate; review both sections before publishing.

## Release checklist

1. Run formatting, clippy, and tests; verify the picker, completion, and clipboard behavior on supported macOS and Linux environments, including SSH and tmux.
2. Set the Cargo version using Cargo tooling, update the changelog and links, and review the English and Chinese README together.
3. Before the first public release, configure tag-triggered GitHub Actions to build versioned platform archives and combine the two release-note sections. A failed build must not publish a complete release.
4. Verify `cargo package`, crate metadata, archive names and layout, and cargo-binstall discovery. Add `[package.metadata.binstall]` only if the release layout requires it.
5. Commit the release preparation through the normal branch and PR flow, then create its immutable tag from the accepted `main` commit.
6. Publish the crate and release only with explicit authorization. Verify the actual published crate, release notes, assets, and a clean cargo-binstall installation.
7. After verifying the first published crate, add crates.io version badges to both README files.

Release automation and cargo-binstall integration are deferred until the local implementation has been tried and accepted. Do not publish a crate merely to reserve its name.
