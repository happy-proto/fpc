# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Complete paths through the shell's native menu, preferring case-insensitive prefixes and falling back to nucleo fuzzy matching. Queries such as `readmemd` and `rdm` can match `README.md`. Search one directory level at a time and show hidden names only for names beginning with `.`.

### Added

- Highlight matched characters in picker results, including nonconsecutive fuzzy matches. Scroll long paths to reveal the matching region and mark clipped ends with ellipses.

## [0.0.2] - 2026-10-12

### Changed

- Use a bottom-up fuzzy picker layout, with the best match nearest the status and search prompt and arrow keys following the displayed direction.
- Edit search queries at a visible cursor with character and word movement, Home/End and Ctrl+A/E, word deletion, and Ctrl+Y to paste deleted text. Insert typed or pasted text at the cursor and scroll long queries to keep it visible; character movement and deletion preserve whole Unicode graphemes.

## [0.0.1] - 2026-10-11

### Added

- Copy absolute file and directory paths directly or select them with an interactive fuzzy picker. Symbolic links retain their own paths, including dangling links.
- Include hidden and ignored paths by default, with optional Git-style ignore rules and custom ignore files.
- Copy through the system clipboard locally, OSC 52 over SSH, or tmux's clipboard channel. Successful copies are silent; unavailable channels print a warning and the path. Use `--print` for explicit path output.
- Dynamic Bash, Fish, and Zsh completion for paths, options, and shell names, with explicit installation into standalone files without editing shell startup files.
- Detailed `--version` output with source commit, dirty state, and build information, plus a short `-V` form.

[Unreleased]: https://github.com/happy-proto/fpc/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/happy-proto/fpc/releases/tag/v0.0.2
[0.0.1]: https://github.com/happy-proto/fpc/releases/tag/v0.0.1
