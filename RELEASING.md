# Releasing NetScope

Releases are cut from a tag. Pushing a `v*` tag builds every platform and
opens a **draft** GitHub release with the bundles attached; nothing is public
until you publish that draft yourself.

## What a release contains

| Asset | Built by | Platform |
| --- | --- | --- |
| `.dmg` / `.app.tar.gz` (aarch64) | `release.yml` | macOS, Apple Silicon |
| `.dmg` / `.app.tar.gz` (x86_64) | `release.yml` | macOS, Intel |
| `.msi` / `.exe` | `release.yml` | Windows x64 |
| `.deb` / `.AppImage` / `.rpm` | `release.yml` | Linux x64 |
| `netscope-extension-<version>.zip` | `release.yml` | Browser extension |

## Cutting a release

1. **Start from a clean `main`.**

   ```bash
   git checkout main && git pull
   ```

2. **Bump the version.** This rewrites `package.json`, `src-tauri/tauri.conf.json`,
   `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` in one step, so the tag can
   never disagree with the bundles it produces.

   ```bash
   npm run version:set 0.2.0
   # the extension is versioned independently; bump it only when it changed:
   npm run version:set 0.2.0 -- --extension 1.1.0
   ```

3. **Update `CHANGELOG.md`.** Move the `Unreleased` entries under a new
   `## [0.2.0] - YYYY-MM-DD` heading and refresh the link definitions at the
   bottom.

4. **Commit and push.**

   ```bash
   git commit -am "release 0.2.0"
   git push origin main
   ```

5. **Tag and push the tag.** The tag drives the whole pipeline.

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

6. **Watch the run** under Actions → Release. The build refuses to continue if
   the tag and the manifests disagree, so a mismatch fails fast rather than
   shipping a mislabelled bundle.

7. **Publish the draft.** Under Releases, review the attached assets and the
   notes, then publish. Releases are drafts by default precisely so a failed
   platform build can be caught before anyone downloads it.

## Re-running a build

If a platform fails for an infrastructure reason, re-run the job from the
Actions UI, or use **Run workflow** on the Release workflow and pass the
existing tag. Assets are uploaded with `--clobber`, so a re-run replaces the
partial upload rather than duplicating it.

## Pre-releases

For a beta, tag it the same way (`v0.2.0-rc.1` is not accepted by the version
check, so use a normal `vX.Y.Z` tag) and tick **Set as a pre-release** on the
draft before publishing.

## macOS signing and notarization

The workflow signs and notarizes macOS bundles when these repository secrets
are present, and produces unsigned bundles when they are not:

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 of the Developer ID `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Notarization credentials |

Unsigned macOS builds trigger a Gatekeeper warning on first launch, so add
these before advertising downloads widely.

## Versioning

- **Patch** — fixes only, no new views or commands.
- **Minor** — new capability (a view, a scanner, a detection source).
- **Major** — reserved for changes that break stored scan data or the
  extension ↔ app protocol.

The browser extension carries its own version because it ships to browser
stores on its own schedule. Bump it with `--extension` whenever anything under
`extension/` changes.
