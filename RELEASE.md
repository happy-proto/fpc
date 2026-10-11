# Release runbook

## Prepare

1. Create a release branch from `main` and choose a [semantic version](https://semver.org/spec/v2.0.0.html). The first release is `0.0.1`.
2. Update the package version with `cargo set-version <version>` if needed, then run `cargo check` to refresh the lockfile.
3. Review `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Move the release's changes into `## [<version>] - YYYY-MM-DD` using the publication date, retain an empty `Unreleased` section, and update the release/comparison links. Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
4. Run `mise run check`, `mise run test`, `cargo package --locked`, and `./scripts/release_notes.py v<version>`. Review the extracted highlights and update both README files if usage changed.
5. Open an English PR, wait for all checks to pass, and squash-merge it into `main`.

## Publish

1. Confirm the accepted commit, Cargo version, changelog, and date. Create and push the matching annotated tag:

   ```sh
   git tag -a v<version> <accepted-commit> -m "Release v<version>"
   git push origin v<version>
   ```

2. Wait for the Release workflow to finish, including **Publish to crates.io**. After package validation and all target builds succeed, it creates the GitHub release and publishes the tagged package to crates.io using OIDC Trusted Publishing. Review the published assets and notes: the version's maintained changelog is followed by GitHub's generated changes since the previous stable release. CI extracts these notes; maintainers write the changelog before tagging.
3. Confirm the intended version appears on crates.io. Publishing does not require local `cargo login` or a long-lived GitHub secret. Pull requests, nightly builds, and manual workflow dispatches do not publish crates.
4. Verify installation with `cargo binstall fpc --version <version> --no-confirm` and `cargo install fpc --version <version> --locked` in isolated installation directories. Check the binary's version and a representative copy operation. After the first crates.io publication, add its badges to both README files.

If a step fails, inspect the existing release, draft, tag, artifacts, and registry before retrying only missing steps. Never move a published stable tag, replace its assets, or reuse a crate version; publish corrections as a new version.

If the GitHub release exists but **Publish to crates.io** failed, fix the authentication or registry issue and rerun only the failed job. Do not rerun the successful GitHub release job. If Cargo timed out after uploading, check crates.io before retrying; the upload may have succeeded.

## One-time Trusted Publishing setup

crates.io requires an existing crate before [Trusted Publishing](https://crates.io/docs/trusted-publishing) can be configured. The empty 0.0.0 bootstrap release is published locally with an API token; it contains no CLI. The first functional release, 0.0.1, and subsequent releases use OIDC in CI.

After the bootstrap publication, an owner must add a GitHub Trusted Publisher in the [crate settings](https://crates.io/crates/fpc/settings):

- Repository owner: `happy-proto`
- Repository name: `fpc`
- Workflow filename: `release.yml`
- Environment: leave empty (the publishing job does not use a GitHub environment)

The workflow grants `id-token: write` only to the tagged crate publishing job. `rust-lang/crates-io-auth-action` exchanges the GitHub identity token for a short-lived crates.io token and revokes it when the job completes. Do not add a permanent crates.io token to GitHub secrets.

The 0.0.0 bootstrap is not a functional release and does not need a matching GitHub release or entry in the CLI changelog.
