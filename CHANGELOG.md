# Changelog

All notable changes to NetScope are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Release pipeline: tagged builds produce macOS (Apple Silicon and Intel),
  Windows and Linux bundles plus a packaged browser extension.
- CI on pull requests: frontend typecheck/build, `cargo check`, and a guard
  that the version is identical across every manifest.
- `npm run version:set <version>` to bump `package.json`, `tauri.conf.json`,
  `Cargo.toml` and `Cargo.lock` together.

### Fixed

- `npm ci` failed on a peer dependency conflict between `vite` 8 and
  `@vitejs/plugin-vue` 5; the plugin is now on `^6`, which supports vite 8.

## [0.1.0] - Unreleased

First tagged release, covering the work already on `main`.

### Added

- Dashboard with traffic, defender and hard disk overview cards.
- Outbound, inbound and browser traffic views with sorting, grouping and
  per-connection detail panes.
- Alerts triage: suspicious traffic is routed to the alerts tab with
  suggested remediation for each issue.
- Defender: antivirus status, scan reports stored on disk, threat detail view
  and false-positive filtering.
- Intrusion detection with detail view and mitigation actions.
- File scan: locate files by name across the machine and inspect them, with
  malware lookups against external reputation services.
- Browser extension that captures network traffic and forwards it to the
  desktop app.

[Unreleased]: https://github.com/k-antwi/netscope/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/k-antwi/netscope/releases/tag/v0.1.0
