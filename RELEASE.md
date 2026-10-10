# Release runbook

Use this procedure for each stable release. Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html); published versions, tags, and release assets are immutable.

## Prepare the release PR

1. Start a release branch from the current `main`. Review changes since the previous release and choose the next version; the first release is `0.0.1`.
2. Set the version with `cargo set-version <version>` when it changes, then run `cargo check` to update the lockfile. Do not edit version fields or the lockfile manually.
3. Move the relevant `Unreleased` entries in `CHANGELOG.md` into `## [<version>] - YYYY-MM-DD`, using the actual planned publication date. Keep an `Unreleased` section above it and omit empty categories. Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
4. Link the version heading to its GitHub release. Update the `Unreleased` link to compare the new tag against `HEAD`; for later versions, include a comparison with the preceding tag.
5. Review installation instructions and both README files when behavior or supported platforms change. Run `mise run check`, `mise run test`, `cargo package --locked`, and `./scripts/release_notes.py v<version>`.
6. Open an English PR to `main`. Require the fast checks, complete shell tests, native Ghostty clipboard acceptance on Linux and macOS, and all four release archive builds to pass. Inspect the artifacts and resolve failures before squash-merging.

The PR and manual Release workflow only build and upload artifacts. They do not publish. `scripts/release_notes.py` validates the tag/version/date/link agreement and extracts the maintained notes; an `Unreleased`-only changelog intentionally fails release validation.

## Publish

Only proceed with explicit authorization to publish the prepared version.

1. Refresh `main` and check that the intended release commit and its checks are the accepted ones. Confirm the date, Cargo version, tag, and maintained notes still agree. If preparation changed, use another PR before tagging.
2. Create and push the annotated tag from that accepted commit:

   ```sh
   git tag -a v<version> <accepted-commit> -m "Release v<version>"
   git push origin v<version>
   ```

3. Watch the Release workflow. It validates the tag against `main`, packages Linux AMD64/ARM64 and macOS AMD64/ARM64 binaries, and publishes only after every build succeeds. Check the release contains all four archives, `SHA256SUMS`, maintained **Highlights**, and **GitHub-generated changes**.
4. From the same tagged checkout, run `cargo publish --dry-run --locked`, review the package, then run `cargo publish --locked` with the authorized crates.io account. Confirm the registry reports the intended version.

## Verify the published version

1. Download the archives and verify `SHA256SUMS`. Check the executable version and exercise a representative copy operation on supported platforms.
2. In an isolated installation directory, verify `cargo binstall fpc --version <version> --no-confirm --install-path <directory>` installs the release archive rather than falling back to a source build. Verify its version and basic behavior.
3. Verify `cargo install fpc --version <version> --locked --root <directory>` works from crates.io. After the first successful publication, add crates.io badges to both README files through a normal PR.
4. Confirm `Unreleased` is ready to record subsequent changes.

## Handle a failed or partial release

- Fix preparation or build failures before publication. A failed build must not publish a complete release.
- If publication fails, inspect the tag, workflow artifacts, GitHub release or draft, and crates.io before retrying. Retry only missing steps; do not assume a failed command made no changes.
- Do not move a published tag, replace published assets, or reuse a published crate version. Ship corrections as a new version through the same procedure.
