# Agent guidance

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before making changes.
- Follow [RELEASE.md](RELEASE.md) when preparing or publishing a version.
- Maintain [CHANGELOG.md](CHANGELOG.md) as net changes to CLI behavior since the previous release. Exclude development history, CI, tests, tooling, packaging, installer integration, and release automation; describe those in their relevant docs. Keep unpublished changes under `Unreleased`; rolling nightly release notes include the entire section.
- Use English for project artifacts, including pull request and issue titles and descriptions. Keep [README.zh-CN.md](README.zh-CN.md) aligned with [README.md](README.md).
- Prefer integration and end-to-end tests that exercise public interfaces and lock in user-observable behavior. Use unit tests sparingly for boundaries that are difficult or costly to cover at those levels; avoid coupling tests to private helpers, internal data structures, or a particular implementation.
