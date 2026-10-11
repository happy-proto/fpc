#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "typer>=0.27.3",
# ]
# ///

import os
import subprocess
import tempfile
from pathlib import Path

import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)


@app.command(
    help="Verify release validation with isolated package and changelog fixtures."
)
def main() -> None:
    script = Path(__file__).resolve().parents[2] / "scripts/release.py"
    with tempfile.TemporaryDirectory(prefix="fpc-release-") as directory:
        root = Path(directory)
        manifest = '[package]\nname = "fpc"\nversion = "0.0.1"\nrepository = "https://github.com/happy-proto/fpc"\n'
        changelog = """# Changelog

## [Unreleased]

## [0.0.1] - 2020-01-01

### Added

- First release.

## [0.0.0] - 2019-01-01

### Fixed

- Older changes.

[Unreleased]: https://github.com/happy-proto/fpc/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/happy-proto/fpc/releases/tag/v0.0.1
"""
        cases = [
            ("valid release", "v0.0.1", manifest, changelog, None),
            ("tag mismatch", "v0.0.2", manifest, changelog, "does not match"),
            (
                "version comparison is numeric",
                "v0.0.10",
                manifest.replace('"0.0.1"', '"0.0.10"'),
                changelog.replace("0.0.1", "0.0.10").replace("0.0.0", "0.0.9"),
                None,
            ),
            ("tag without prefix", "0.0.1", manifest, changelog, "vVERSION"),
            (
                "tag with leading zero",
                "v0.0.01",
                manifest,
                changelog,
                "stable semantic",
            ),
            (
                "tag with incomplete version",
                "v0.0",
                manifest,
                changelog,
                "stable semantic",
            ),
            (
                "unreleased changes",
                "v0.0.1",
                manifest,
                changelog.replace(
                    "## [Unreleased]\n",
                    "## [Unreleased]\n\n### Added\n\n- Pending change.\n",
                ),
                "Unreleased section must be empty",
            ),
            (
                "unreleased explanation",
                "v0.0.1",
                manifest,
                changelog.replace(
                    "## [Unreleased]\n",
                    "## [Unreleased]\n\nDeferred until the next release.\n",
                ),
                "Unreleased section must be empty",
            ),
            (
                "higher version later in changelog",
                "v0.0.1",
                manifest,
                changelog.replace("[0.0.0]", "[0.0.2]"),
                "newest version",
            ),
            (
                "release section is not first",
                "v0.0.1",
                manifest,
                changelog.replace("[0.0.1] - 2020", "[0.0.0] - 2020").replace(
                    "[0.0.0] - 2019", "[0.0.1] - 2019"
                ),
                "newest version",
            ),
            ("unreleased only", "v0.0.1", manifest, "## [Unreleased]\n", "exactly one"),
            (
                "missing date",
                "v0.0.1",
                manifest,
                changelog.replace(" - 2020-01-01", ""),
                "needs a",
            ),
            (
                "invalid date",
                "v0.0.1",
                manifest,
                changelog.replace("2020-01-01", "2020-02-30"),
                "day",
            ),
            (
                "future date",
                "v0.0.1",
                manifest,
                changelog.replace("2020-01-01", "2999-01-01"),
                "future",
            ),
            (
                "missing changes",
                "v0.0.1",
                manifest,
                changelog.replace("- First release.", ""),
                "change entry",
            ),
            (
                "wrong link",
                "v0.0.1",
                manifest,
                changelog.replace("releases/tag/v0.0.1", "releases/tag/v0.0.2"),
                "version link",
            ),
            (
                "duplicate version",
                "v0.0.1",
                manifest,
                changelog.replace("[0.0.0]", "[0.0.1]"),
                "exactly one",
            ),
            (
                "unstable version",
                "v0.0.1-alpha.1",
                manifest.replace('"0.0.1"', '"0.0.1-alpha.1"'),
                changelog,
                "stable semantic",
            ),
        ]
        for name, tag, cargo, changes, error in cases:
            (root / "Cargo.toml").write_text(cargo)
            (root / "CHANGELOG.md").write_text(changes)
            result = subprocess.run(
                [
                    "uv",
                    "run",
                    "--quiet",
                    "--script",
                    str(script),
                    tag,
                    "--root",
                    str(root),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            if error:
                assert result.returncode != 0 and error in result.stderr, (name, result)
                assert not result.stdout, (name, result.stdout)
            else:
                assert result.returncode == 0 and not result.stderr, (name, result)
                assert (
                    result.stdout
                    == "## Highlights\n\n### Added\n\n- First release.\n\n## GitHub-generated changes\n\n"
                ), result.stdout
        commit = "0123456789abcdef" * 2 + "01234567"
        changes = "### Added\n\n- First nightly change.\n- Second nightly change.\n\n### Fixed\n\n- Later fix."
        nightly_changelog = changelog.replace(
            "## [Unreleased]\n", f"## [Unreleased]\n\n{changes}\n"
        )
        nightly_cases = [
            ("cumulative nightly", nightly_changelog, commit, [], changes, None),
            ("empty unreleased", changelog, commit, [], "", None),
            (
                "initial nightly",
                f"## [Unreleased]\n\n{changes}\n\n[Unreleased]: https://example.com\n",
                commit,
                [],
                changes,
                None,
            ),
            ("missing unreleased", "# Changelog\n", commit, [], "", "Unreleased"),
            (
                "duplicate unreleased",
                nightly_changelog + "\n## [Unreleased]\n",
                commit,
                [],
                "",
                "exactly one",
            ),
            ("short commit", changelog, "0123456", [], "", "40"),
            ("invalid commit", changelog, "z" * 40, [], "", "hexadecimal"),
            ("tag and nightly", changelog, commit, ["v0.0.1"], "", "combined"),
            (
                "package check and nightly",
                changelog,
                commit,
                ["--check-package"],
                "",
                "combined",
            ),
        ]
        (root / "Cargo.toml").write_text(manifest)
        for name, content, revision, options, expected, error in nightly_cases:
            (root / "CHANGELOG.md").write_text(content)
            result = subprocess.run(
                [
                    str(script),
                    "--nightly-commit",
                    revision,
                    "--root",
                    str(root),
                    *options,
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            if error:
                assert result.returncode != 0 and error in result.stderr, (name, result)
                assert not result.stdout, (name, result.stdout)
            else:
                assert result.returncode == 0 and not result.stderr, (name, result)
                assert result.stdout == (
                    f"# Nightly build\n\nCommit: {commit}\n\n"
                    f"## Unreleased changes\n\n{expected}\n"
                ), (name, result.stdout)
        (root / "CHANGELOG.md").unlink()
        result = subprocess.run(
            [
                "uv",
                "run",
                "--quiet",
                "--script",
                str(script),
                "--root",
                str(root),
                "--check-package",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0 and "fpc 0.0.1" in result.stdout, result
        (root / "Cargo.toml").write_text(
            manifest.replace('name = "fpc"', 'name = "other"')
        )
        result = subprocess.run(
            [
                "uv",
                "run",
                "--quiet",
                "--script",
                str(script),
                "--root",
                str(root),
                "--check-package",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode != 0 and "must be named fpc" in result.stderr, result
        result = subprocess.run(
            ["uv", "run", "--quiet", "--script", str(script), "--help"],
            capture_output=True,
            text=True,
            check=False,
            env=os.environ | {"NO_COLOR": "1", "TERM": "dumb", "COLUMNS": "200"},
        )
        assert result.returncode == 0 and not result.stderr, result
        assert "--nightly-commit" in result.stdout, result.stdout
        assert "FULL_COMMIT_SHA" in result.stdout, result.stdout
        assert "cannot be combined" in result.stdout, result.stdout
    print(f"Release validation: {len(cases) + len(nightly_cases) + 3} scenarios passed")


if __name__ == "__main__":
    app()
