# Release runbook

Successful builds of `main` replace the rolling nightly prerelease. `scripts/release_notes.py --nightly-commit <full-commit-sha>` generates its notes with the source commit and the complete current `Unreleased` section, excluding older releases and changelog reference links. Changes accumulate until a stable release moves them into its versioned section; nightly notes do not display the Cargo package version.

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

2. Wait for the Release workflow to finish. Review the published assets and notes: the version's maintained changelog is followed by GitHub's generated changes since the previous stable release. CI extracts these notes; maintainers write the changelog before tagging.
3. From the tagged checkout, run `cargo publish --dry-run --locked`, inspect the package, then run `cargo publish --locked`. Confirm the intended version appears on crates.io.
4. Verify installation with `cargo binstall fpc --version <version> --no-confirm` and `cargo install fpc --version <version> --locked` in isolated installation directories. Check the binary's version and a representative copy operation. After the first crates.io publication, add its badges to both README files.

If a step fails, inspect the existing release, draft, tag, artifacts, and registry before retrying only missing steps. Never move a published stable tag, replace its assets, or reuse a crate version; publish corrections as a new version.
