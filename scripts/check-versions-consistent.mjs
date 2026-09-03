// Guards against a half-finished bump landing on main: package.json,
// tauri.conf.json and Cargo.toml must all carry the same version.
import { currentVersions } from './version-files.mjs'

const versions = currentVersions()
const unique = new Set(Object.values(versions))

if (unique.size > 1) {
  console.error('version mismatch between manifests:')
  for (const [file, version] of Object.entries(versions)) console.error(`  ${file}: ${version}`)
  console.error('\nRun `npm run version:set <version>` to bring them back in line.')
  process.exit(1)
}

console.log(`all manifests at ${[...unique][0]}`)
