// Sets the app version across package.json, tauri.conf.json, Cargo.toml and
// Cargo.lock so a release tag can never disagree with the bundles it produces.
// Usage: node scripts/bump-version.mjs 0.2.0 [--extension 1.1.0]
import { currentVersions, extensionVersion, isSemver, setAppVersion, setExtensionVersion } from './version-files.mjs'

const args = process.argv.slice(2)
const version = args[0]?.replace(/^v/, '')
const extFlag = args.indexOf('--extension')
const extVersion = extFlag === -1 ? null : args[extFlag + 1]?.replace(/^v/, '')

if (!version || !isSemver(version)) {
  console.error('usage: node scripts/bump-version.mjs <X.Y.Z> [--extension <X.Y.Z>]')
  process.exit(1)
}
if (extFlag !== -1 && !isSemver(extVersion ?? '')) {
  console.error('--extension needs a X.Y.Z version')
  process.exit(1)
}

setAppVersion(version)
if (extVersion) setExtensionVersion(extVersion)

console.log(`app version -> ${version}`)
for (const [file, v] of Object.entries(currentVersions())) console.log(`  ${file}: ${v}`)
console.log(`extension version -> ${extensionVersion()}`)
console.log('\nNext: update CHANGELOG.md, commit, then `git tag v' + version + '` and push the tag.')
