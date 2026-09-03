// Shared helpers for reading and writing the versions NetScope keeps in sync.
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

export const paths = {
  packageJson: join(root, 'package.json'),
  tauriConf: join(root, 'src-tauri', 'tauri.conf.json'),
  cargoToml: join(root, 'src-tauri', 'Cargo.toml'),
  cargoLock: join(root, 'src-tauri', 'Cargo.lock'),
  manifest: join(root, 'extension', 'manifest.json'),
}

const read = (p) => readFileSync(p, 'utf8')

export function currentVersions() {
  return {
    'package.json': JSON.parse(read(paths.packageJson)).version,
    'src-tauri/tauri.conf.json': JSON.parse(read(paths.tauriConf)).version,
    'src-tauri/Cargo.toml': read(paths.cargoToml).match(/^version = "(.+)"$/m)?.[1],
  }
}

export function extensionVersion() {
  return JSON.parse(read(paths.manifest)).version
}

function writeJsonVersion(path, version) {
  const source = read(path)
  const updated = source.replace(/("version":\s*)"[^"]+"/, `$1"${version}"`)
  if (updated === source) throw new Error(`no version field found in ${path}`)
  writeFileSync(path, updated)
}

export function setAppVersion(version) {
  writeJsonVersion(paths.packageJson, version)
  writeJsonVersion(paths.tauriConf, version)

  // Cargo.toml: only the [package] version, which is the first one in the file.
  const toml = read(paths.cargoToml)
  writeFileSync(paths.cargoToml, toml.replace(/^version = ".+"$/m, `version = "${version}"`))

  // Cargo.lock: the version line directly under the netscope package entry.
  const lock = read(paths.cargoLock)
  const updatedLock = lock.replace(
    /(name = "netscope"\nversion = )"[^"]+"/,
    `$1"${version}"`,
  )
  if (updatedLock === lock) throw new Error('netscope entry not found in Cargo.lock')
  writeFileSync(paths.cargoLock, updatedLock)
}

export function setExtensionVersion(version) {
  writeJsonVersion(paths.manifest, version)
}

export function isSemver(version) {
  return /^\d+\.\d+\.\d+$/.test(version)
}
