// Fails the release build when the tag does not match the versions in the tree.
// Usage: node scripts/check-version.mjs v0.2.0
import { currentVersions, isSemver } from './version-files.mjs'

const raw = process.argv[2]
if (!raw) {
  console.error('usage: node scripts/check-version.mjs <tag>')
  process.exit(1)
}

const tagVersion = raw.replace(/^refs\/tags\//, '').replace(/^v/, '')
if (!isSemver(tagVersion)) {
  console.error(`tag "${raw}" is not a vX.Y.Z release tag`)
  process.exit(1)
}

const versions = currentVersions()
const mismatched = Object.entries(versions).filter(([, v]) => v !== tagVersion)

if (mismatched.length > 0) {
  console.error(`tag ${raw} expects version ${tagVersion}, but found:`)
  for (const [file, version] of mismatched) console.error(`  ${file}: ${version}`)
  console.error('\nRun `npm run version:set <version>` and commit before tagging.')
  process.exit(1)
}

console.log(`version ${tagVersion} matches every manifest`)
