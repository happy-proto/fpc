# Release runbook

## Prepare

1. Create a release branch from `main` and choose a [semantic version](https://semver.org/spec/v2.0.0.html).
2. Update the package version with `cargo set-version <version>` if needed, then run `cargo check` to refresh the lockfile.
3. Review `Unreleased` in [CHANGELOG.md](CHANGELOG.md). Move the release's changes into `## [<version>] - YYYY-MM-DD` using the publication date, retain an empty `Unreleased` section, and update the release/comparison links. Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
4. Run `mise run check`, `mise run test`, `cargo package --locked`, and `./scripts/release.py v<version>`. Review the extracted highlights and update both README files if usage changed.
5. Open an English PR, wait for all checks to pass, and squash-merge it into `main`.

## Publish

1. Confirm the accepted commit, Cargo version, changelog, and date. Create and push the matching annotated tag:

   ```sh
   git tag -a v<version> <accepted-commit> -m "Release v<version>"
   git push origin v<version>
   ```

2. Wait for the Release workflow to finish, including **Publish to crates.io**. Review the published GitHub release, assets, and notes.
3. Confirm the intended version appears on crates.io.
4. Verify installation with `cargo binstall fpc --version <version> --no-confirm` and `cargo install fpc --version <version> --locked` in isolated installation directories. Check the binary's version and a representative copy operation. After the first crates.io publication, add its badges to both README files.

If a step fails, inspect the existing release, draft, tag, artifacts, and registry before retrying only missing steps. Never move a published stable tag, replace its assets, or reuse a crate version; publish corrections as a new version.

If the GitHub release exists but **Publish to crates.io** failed, rerun only the failed job after resolving the failure. Do not rerun the successful GitHub release job. If Cargo timed out after uploading, check crates.io before retrying; the upload may have succeeded.
