# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> None:
    script = Path(__file__).resolve().parents[2] / "scripts/release_notes.py"
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
                [sys.executable, str(script), tag, "--root", str(root)],
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
        (root / "Cargo.toml").write_text(manifest)
        (root / "CHANGELOG.md").unlink()
        result = subprocess.run(
            [sys.executable, str(script), "--root", str(root), "--check-package"],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0 and "fpc 0.0.1" in result.stdout, result
        (root / "Cargo.toml").write_text(
            manifest.replace('name = "fpc"', 'name = "other"')
        )
        result = subprocess.run(
            [sys.executable, str(script), "--root", str(root), "--check-package"],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode != 0 and "must be named fpc" in result.stderr, result
    print(f"Release validation: {len(cases) + 2} scenarios passed")


if __name__ == "__main__":
    main()
