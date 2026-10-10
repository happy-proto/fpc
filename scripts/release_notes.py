#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "typer>=0.27.3",
# ]
# ///

import datetime
import re
import sys
import tomllib
from pathlib import Path
from typing import Annotated

import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)
DEFAULT_ROOT = Path(__file__).resolve().parents[1]


@app.command(help="Validate a release and extract its maintained notes.")
def main(
    tag: Annotated[
        str | None,
        typer.Argument(
            metavar="vVERSION",
            help="Release tag matching Cargo's stable semantic version; required unless --check-package is used.",
        ),
    ] = None,
    check_package: Annotated[
        bool,
        typer.Option(
            "--check-package",
            help="Validate package identity and version without released notes; cannot be combined with a tag.",
        ),
    ] = False,
    root: Annotated[
        Path,
        typer.Option(
            "--root",
            exists=True,
            file_okay=False,
            resolve_path=True,
            help="Repository directory containing Cargo.toml and CHANGELOG.md; defaults to this script's repository.",
        ),
    ] = DEFAULT_ROOT,
) -> None:
    package = tomllib.loads((root / "Cargo.toml").read_text())["package"]
    version = package["version"]
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("Releases require a stable semantic version")
    if package["name"] != "fpc":
        raise ValueError("The released crate and binary must be named fpc")
    if check_package:
        if tag:
            raise typer.BadParameter(
                "--check-package cannot be combined with a release tag"
            )
        print(f"Package ready for release preparation: fpc {version}")
        return
    if tag != f"v{version}":
        raise ValueError(f"Tag {tag!r} does not match Cargo version {version}")
    changelog = (root / "CHANGELOG.md").read_text()
    headings = list(re.finditer(r"^## \[([^\]]+)\](.*)$", changelog, re.MULTILINE))
    if not headings or headings[0][1] != "Unreleased":
        raise ValueError("The changelog must start with an Unreleased section")
    matches = [heading for heading in headings if heading[1] == version]
    if len(matches) != 1:
        raise ValueError(f"Expected exactly one changelog section for {version}")
    heading = matches[0]
    if headings.index(heading) != 1:
        raise ValueError(
            "The released version must be the newest version in the changelog"
        )
    date_match = re.fullmatch(r" - (\d{4}-\d{2}-\d{2})", heading[2])
    if not date_match:
        raise ValueError("The released changelog section needs a YYYY-MM-DD date")
    date = datetime.date.fromisoformat(date_match[1])
    if date > datetime.datetime.now(datetime.UTC).date():
        raise ValueError("The release date cannot be in the future")
    following = [item.start() for item in headings if item.start() > heading.start()]
    end = following[0] if following else len(changelog)
    notes = changelog[heading.end() : end]
    notes = re.split(r"^\[[^\]]+\]:", notes, maxsplit=1, flags=re.MULTILINE)[0].strip()
    if not re.search(
        r"^### (Added|Changed|Deprecated|Removed|Fixed|Security)$", notes, re.MULTILINE
    ):
        raise ValueError("Release notes need a Keep a Changelog category")
    if not re.search(r"^- \S", notes, re.MULTILINE):
        raise ValueError("Release notes must contain a change entry")
    link = f"{package['repository']}/releases/tag/{tag}"
    if f"[{version}]: {link}" not in changelog.splitlines():
        raise ValueError("The changelog version link must point to this GitHub release")
    print(f"## Highlights\n\n{notes}\n\n## GitHub-generated changes\n")


if __name__ == "__main__":
    try:
        app()
    except (ValueError, KeyError, OSError) as error:
        print(f"Release validation failed: {error}", file=sys.stderr)
        sys.exit(1)
