# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

import argparse
import datetime
import re
import sys
import tomllib
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate a release and extract its maintained notes"
    )
    parser.add_argument("tag", nargs="?", help="Release tag, in the form v<version>")
    parser.add_argument(
        "--check-package",
        action="store_true",
        help="Validate package identity and version without requiring released notes",
    )
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    args = parser.parse_args()
    package = tomllib.loads((args.root / "Cargo.toml").read_text())["package"]
    version = package["version"]
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("Releases require a stable semantic version")
    if package["name"] != "fpc":
        raise ValueError("The released crate and binary must be named fpc")
    if args.check_package:
        if args.tag:
            parser.error("--check-package cannot be combined with a release tag")
        print(f"Package ready for release preparation: fpc {version}")
        return
    if args.tag != f"v{version}":
        raise ValueError(f"Tag {args.tag!r} does not match Cargo version {version}")
    changelog = (args.root / "CHANGELOG.md").read_text()
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
    link = f"{package['repository']}/releases/tag/{args.tag}"
    if f"[{version}]: {link}" not in changelog.splitlines():
        raise ValueError("The changelog version link must point to this GitHub release")
    print(f"## Highlights\n\n{notes}\n\n## GitHub-generated changes\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(f"Release validation failed: {error}", file=sys.stderr)
        sys.exit(1)
