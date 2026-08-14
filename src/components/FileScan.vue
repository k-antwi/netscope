<script setup lang="ts">
import { ref, computed, watchEffect, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import ScanDialog from './ScanDialog.vue'
import FileDetail from './FileDetail.vue'
import type { FileMatch, FileDetails, FileScanResult, ScanProgress, ScanSummary } from '../types'

const emit = defineEmits<{ summary: [ScanSummary] }>()

const dialogOpen = ref(false)
const query = ref('')
const isScanning = ref(false)
const result = ref<FileScanResult | null>(null)
const progress = ref<ScanProgress | null>(null)
const error = ref('')
const filter = ref('')
const copiedPath = ref('')
const selected = ref<Set<string>>(new Set())
const confirmDelete = ref(false)
const isDeleting = ref(false)
const deleteErrors = ref<string[]>([])
const selectedFile = ref<FileMatch | null>(null)
const fileDetails = ref<FileDetails | null>(null)
const isLoadingDetails = ref(false)

let unlistenProgress: UnlistenFn | null = null
let copiedTimer: ReturnType<typeof setTimeout> | null = null

type SortKey = 'name' | 'path' | 'size' | 'modified'
const sortKey = ref<SortKey>('path')
const sortAsc = ref(true)

const matches = computed(() => result.value?.matches ?? [])

const selectedItems = computed(() =>
  result.value ? result.value.matches.filter((m: FileMatch) => selected.value.has(m.path)) : []
)

const allVisibleSelected = computed(() =>
  visible.value.length > 0 && visible.value.every((m: FileMatch) => selected.value.has(m.path))
)

const someVisibleSelected = computed(() =>
  visible.value.some((m: FileMatch) => selected.value.has(m.path)) && !allVisibleSelected.value
)

const visible = computed(() => {
  const q = filter.value.toLowerCase()
  const rows = q ? matches.value.filter(m => m.path.toLowerCase().includes(q)) : [...matches.value]
  return rows.sort((a, b) => {
    let cmp: number
    if (sortKey.value === 'size') cmp = a.size - b.size
    else if (sortKey.value === 'modified') cmp = (a.modified ?? 0) - (b.modified ?? 0)
    else cmp = a[sortKey.value].localeCompare(b[sortKey.value])
    return sortAsc.value ? cmp : -cmp
  })
})

function setSort(key: SortKey) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = true
  }
}

function sortIcon(key: SortKey): string {
  if (sortKey.value !== key) return ''
  return sortAsc.value ? ' ↑' : ' ↓'
}

function openDialog() {
  if (isScanning.value) return
  dialogOpen.value = true
}

async function startScan(name: string) {
  dialogOpen.value = false
  query.value = name
  error.value = ''
  filter.value = ''
  result.value = null
  selected.value = new Set()
  selectedFile.value = null
  fileDetails.value = null
  progress.value = { scanned_dirs: 0, found: 0, current: '' }
  isScanning.value = true

  await stopListening()
  unlistenProgress = await listen<ScanProgress>('file-scan-progress', e => {
    progress.value = e.payload
  })

  try {
    result.value = await invoke<FileScanResult>('scan_files', { query: name, limit: 1000 })
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e)
  } finally {
    isScanning.value = false
    progress.value = null
    await stopListening()
  }
}

async function stopScan() {
  await invoke('cancel_file_scan')
}

function toggleSelect(path: string, e: Event) {
  e.stopPropagation()
  const next = new Set(selected.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  selected.value = next
}

function toggleSelectAll() {
  if (allVisibleSelected.value) {
    const next = new Set(selected.value)
    visible.value.forEach((m: FileMatch) => next.delete(m.path))
    selected.value = next
  } else {
    const next = new Set(selected.value)
    visible.value.forEach((m: FileMatch) => next.add(m.path))
    selected.value = next
  }
}

async function selectFile(m: FileMatch) {
  if (selectedFile.value?.path === m.path) {
    selectedFile.value = null
    fileDetails.value = null
    return
  }
  selectedFile.value = m
  fileDetails.value = null
  isLoadingDetails.value = true
  try {
    fileDetails.value = await invoke<FileDetails>('get_file_details', { path: m.path })
  } catch {
    // leave fileDetails null; panel shows path info without extras
  } finally {
    isLoadingDetails.value = false
  }
}

interface DeleteResponse {
  deleted: string[]
  failed: { path: string; error: string }[]
}

async function executeDelete() {
  isDeleting.value = true
  deleteErrors.value = []
  try {
    const paths = [...selected.value]
    const res = await invoke<DeleteResponse>('delete_files', { paths })
    if (res.deleted.length && result.value) {
      const deletedSet = new Set(res.deleted)
      result.value = { ...result.value, matches: result.value.matches.filter((m: FileMatch) => !deletedSet.has(m.path)) }
      selected.value = new Set([...selected.value].filter(p => !deletedSet.has(p)))
    }
    if (res.failed.length) {
      deleteErrors.value = res.failed.map((f: { path: string; error: string }) => `${f.path}: ${f.error}`)
    } else {
      confirmDelete.value = false
    }
  } catch (e) {
    deleteErrors.value = [typeof e === 'string' ? e : String(e)]
  } finally {
    isDeleting.value = false
  }
}

async function stopListening() {
  if (unlistenProgress) {
    unlistenProgress()
    unlistenProgress = null
  }
}

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path)
  } catch {
    const el = document.createElement('textarea')
    el.value = path
    el.style.position = 'fixed'
    el.style.opacity = '0'
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    document.body.removeChild(el)
  }
  copiedPath.value = path
  if (copiedTimer) clearTimeout(copiedTimer)
  copiedTimer = setTimeout(() => (copiedPath.value = ''), 1200)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`
}

function formatDate(secs: number | null): string {
  if (!secs) return '—'
  const d = new Date(secs * 1000)
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: '2-digit' })
}

function formatElapsed(ms: number): string {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`
}

function truncatePath(path: string, max = 70): string {
  return path.length <= max ? path : `…${path.slice(path.length - max + 1)}`
}

function rowClass(m: FileMatch): string {
  const parts = ['row']
  if (m.exact) parts.push('exact')
  if (m.is_dir) parts.push('dir')
  if (m.path === copiedPath.value) parts.push('copied')
  if (selected.value.has(m.path)) parts.push('selected')
  if (selectedFile.value?.path === m.path) parts.push('active')
  return parts.join(' ')
}

watchEffect(() => {
  emit('summary', {
    matches: matches.value.length,
    scannedDirs: result.value?.scanned_dirs ?? progress.value?.scanned_dirs ?? 0,
    elapsedMs: result.value?.elapsed_ms ?? null,
    isScanning: isScanning.value,
    truncated: result.value?.truncated ?? false,
  })
})

onUnmounted(() => {
  if (copiedTimer) clearTimeout(copiedTimer)
  if (isScanning.value) invoke('cancel_file_scan').catch(() => {})
  stopListening()
})

defineExpose({ openDialog })
</script>

<template>
  <div class="file-scan">
    <!-- Scanning -->
    <div v-if="isScanning" class="center-state">
      <span class="spin-lg">↺</span>
      <div class="state-title">Scanning this computer for "{{ query }}"</div>
      <div class="progress-stats">
        <span><b>{{ progress?.found ?? 0 }}</b> found</span>
        <span class="sep">·</span>
        <span><b>{{ (progress?.scanned_dirs ?? 0).toLocaleString() }}</b> folders searched</span>
      </div>
      <div class="progress-path mono">{{ truncatePath(progress?.current || '') || '…' }}</div>
      <button class="btn ghost" @click="stopScan">Stop scan</button>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="center-state">
      <span class="state-icon error">⚠</span>
      <div class="state-title">Scan failed</div>
      <div class="state-sub">{{ error }}</div>
      <button class="btn primary" @click="openDialog">Try again</button>
    </div>

    <!-- Idle / never scanned -->
    <div v-else-if="!result" class="center-state">
      <span class="state-icon">🗂</span>
      <div class="state-title">File scan</div>
      <div class="state-sub">
        Search every readable folder on this computer for a file by name.
      </div>
      <button class="btn primary big" @click="openDialog">Scan file</button>
    </div>

    <!-- Results -->
    <div v-else class="results">
      <div class="results-bar">
        <div class="results-summary">
          <span class="count">{{ matches.length.toLocaleString() }}</span>
          <span class="count-label">
            {{ matches.length === 1 ? 'match' : 'matches' }} for
          </span>
          <span class="query mono">{{ result.query }}</span>
          <span v-if="result.cancelled" class="tag warn">stopped early</span>
          <span v-if="result.truncated" class="tag warn">limit reached</span>
        </div>
        <div class="results-actions">
          <input
            v-if="matches.length"
            v-model="filter"
            class="search"
            placeholder="Filter results by path…"
            spellcheck="false"
          />
          <button
            v-if="selected.size"
            class="btn danger"
            @click="confirmDelete = true; deleteErrors = []"
          >
            Delete {{ selected.size }} {{ selected.size === 1 ? 'item' : 'items' }}
          </button>
          <button class="btn primary" @click="openDialog">Scan file</button>
        </div>
      </div>

      <div class="content">
        <div v-if="!matches.length" class="center-state grow">
          <span class="state-icon">🔍</span>
          <div class="state-title">No files named "{{ result.query }}"</div>
          <div class="state-sub">
            Searched {{ result.scanned_dirs.toLocaleString() }} folders in
            {{ formatElapsed(result.elapsed_ms) }}.
            <template v-if="result.denied">
              {{ result.denied.toLocaleString() }} folders were unreadable.
            </template>
          </div>
          <button class="btn primary" @click="openDialog">Try another name</button>
        </div>

        <div v-else-if="!visible.length" class="center-state grow">
          <span class="state-sub">No result matches the filter "{{ filter }}".</span>
        </div>

        <div v-else class="table-wrap">
          <table>
            <thead>
              <tr>
                <th class="check-col">
                  <input
                    type="checkbox"
                    class="row-check"
                    :checked="allVisibleSelected"
                    :indeterminate="someVisibleSelected"
                    @change="toggleSelectAll"
                    title="Select all visible"
                  />
                </th>
                <th @click="setSort('name')" class="sortable">Name{{ sortIcon('name') }}</th>
                <th @click="setSort('path')" class="sortable">Location{{ sortIcon('path') }}</th>
                <th @click="setSort('size')" class="sortable num">Size{{ sortIcon('size') }}</th>
                <th @click="setSort('modified')" class="sortable">Modified{{ sortIcon('modified') }}</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="m in visible"
                :key="m.path"
                :class="rowClass(m)"
                @click="selectFile(m)"
                :title="m.path"
              >
                <td class="check-col" @click.stop>
                  <input
                    type="checkbox"
                    class="row-check"
                    :checked="selected.has(m.path)"
                    @change="toggleSelect(m.path, $event)"
                  />
                </td>
                <td class="name">
                  <span class="name-cell">
                    <span class="kind">{{ m.is_dir ? '📁' : '📄' }}</span>
                    <span class="name-text">{{ m.name }}</span>
                    <span v-if="m.exact" class="tag exact">exact</span>
                  </span>
                </td>
                <td class="path mono"><bdi>{{ m.parent }}</bdi></td>
                <td class="size mono num">{{ m.is_dir ? '—' : formatSize(m.size) }}</td>
                <td class="modified">{{ formatDate(m.modified) }}</td>
                <td class="action">
                  <button
                    class="copy-btn"
                    :class="{ done: m.path === copiedPath }"
                    @click.stop="copyPath(m.path)"
                    :title="m.path === copiedPath ? 'Copied' : 'Copy full path'"
                  >
                    {{ m.path === copiedPath ? '✓' : '⧉' }}
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <Transition name="panel">
          <FileDetail
            v-if="selectedFile"
            :file="selectedFile"
            :details="fileDetails"
            :is-loading="isLoadingDetails"
            @close="selectedFile = null; fileDetails = null"
          />
        </Transition>
      </div>
    </div>

    <ScanDialog
      :open="dialogOpen"
      :initial="query"
      @search="startScan"
      @close="dialogOpen = false"
    />

    <div v-if="confirmDelete" class="modal-overlay" @click.self="confirmDelete = false; deleteErrors = []">
      <div class="modal">
        <div class="modal-title">
          Delete {{ selected.size }} {{ selected.size === 1 ? 'item' : 'items' }}?
        </div>
        <div class="modal-body">
          <div v-if="selectedItems.some(m => m.is_dir)" class="modal-warn">
            ⚠ Directories will be deleted along with all their contents.
          </div>
          <ul class="delete-list">
            <li v-for="m in selectedItems.slice(0, 8)" :key="m.path" class="delete-path mono">
              {{ m.is_dir ? '📁' : '📄' }} {{ m.path }}
            </li>
            <li v-if="selectedItems.length > 8" class="delete-more">
              …and {{ selectedItems.length - 8 }} more
            </li>
          </ul>
          <div v-if="deleteErrors.length" class="delete-errors">
            <div class="delete-errors-title">Some items could not be deleted:</div>
            <div v-for="e in deleteErrors" :key="e" class="delete-error mono">{{ e }}</div>
          </div>
        </div>
        <div class="modal-footer">
          <button
            class="btn ghost"
            @click="confirmDelete = false; deleteErrors = []"
            :disabled="isDeleting"
          >
            Cancel
          </button>
          <button class="btn danger" @click="executeDelete" :disabled="isDeleting">
            {{ isDeleting ? 'Deleting…' : `Delete ${selected.size} ${selected.size === 1 ? 'item' : 'items'}` }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-scan {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg);
}

/* Centred states */
.center-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 24px;
  text-align: center;
  color: var(--muted);
}
.center-state.grow { flex: 1; }

.state-icon { font-size: 34px; line-height: 1; }
.state-icon.error { color: var(--red); }
.state-title { font-size: 15px; font-weight: 600; color: var(--text); }
.state-sub { font-size: 12px; max-width: 420px; line-height: 1.6; }

@keyframes spin { to { transform: rotate(360deg); } }
.spin-lg { display: inline-block; animation: spin 1s linear infinite; font-size: 26px; }

.progress-stats { display: flex; gap: 6px; font-size: 12px; }
.progress-stats b { color: var(--text); font-variant-numeric: tabular-nums; }
.sep { opacity: 0.5; }

.progress-path {
  font-size: 11px;
  color: var(--muted);
  opacity: 0.7;
  max-width: 620px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: center;
}

/* Buttons */
.btn {
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  cursor: pointer;
  border: 1px solid var(--border);
}
.btn.ghost { background: var(--surface-2); color: var(--text); }
.btn.ghost:hover { border-color: var(--muted); }
.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #0d1117;
  font-weight: 600;
}
.btn.primary:hover { filter: brightness(1.08); }
.btn.big { padding: 10px 20px; font-size: 13px; margin-top: 4px; }
.btn.danger {
  background: var(--red, #da3633);
  border-color: var(--red, #da3633);
  color: #fff;
  font-weight: 600;
}
.btn.danger:hover { filter: brightness(1.1); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* Results */
.results { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.content { display: flex; flex: 1; overflow: hidden; }

.panel-enter-active, .panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-enter-from, .panel-leave-to { transform: translateX(100%); opacity: 0; }

.results-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.results-summary { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--muted); }
.count { font-size: 14px; font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
.query { color: var(--accent); }

.results-actions { display: flex; align-items: center; gap: 8px; }

.search {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 5px 10px;
  color: var(--text);
  font-size: 12px;
  width: 220px;
  outline: none;
}
.search:focus { border-color: var(--accent); }
.search::placeholder { color: var(--muted); }

.tag {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  white-space: nowrap;
  flex-shrink: 0;
}
.tag.warn { color: var(--orange); border-color: rgba(210, 153, 34, 0.3); background: rgba(210, 153, 34, 0.12); }
.tag.exact { color: var(--green); border-color: rgba(63, 185, 80, 0.3); background: var(--green-dim); }

/* Table */
.table-wrap { flex: 1; overflow-y: auto; }

table { width: 100%; border-collapse: collapse; font-size: 12px; table-layout: fixed; }

thead { position: sticky; top: 0; z-index: 1; background: var(--surface); }

th {
  padding: 8px 12px;
  text-align: left;
  font-weight: 500;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}
th.sortable { cursor: pointer; user-select: none; }
th.sortable:hover { color: var(--text); }
th.num { text-align: right; }

td {
  padding: 7px 12px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.5);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row { cursor: pointer; transition: background 0.1s; }
.row:hover { background: var(--surface-2); }
.row.exact { background: var(--green-dim); }
.row.exact:hover { background: rgba(63, 185, 80, 0.18); }
.row.dir { color: var(--muted); }
.row.copied { outline: 1px solid var(--accent); }

.name { width: 30%; }
.name-cell { display: flex; align-items: center; gap: 6px; min-width: 0; }
.name-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.kind { flex-shrink: 0; font-size: 11px; }

/* RTL flow truncates long paths at the front ("…/Documents/archive"); the
   isolated <bdi> keeps the path itself reading left-to-right. */
.path { color: var(--muted); direction: rtl; text-align: left; }
.path bdi { unicode-bidi: isolate; }
.size { width: 90px; color: var(--muted); }
.num { text-align: right; }
.modified { width: 110px; color: var(--muted); font-size: 11px; }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }

.action { width: 36px; text-align: center; }
.copy-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 13px;
  padding: 2px 4px;
  border-radius: 4px;
  line-height: 1;
}
.copy-btn:hover { color: var(--accent); background: var(--surface-2); }
.copy-btn.done { color: var(--green); }

/* Checkbox column */
.check-col { width: 36px; text-align: center; padding: 7px 8px; }
thead .check-col { padding: 8px 8px; }

.row-check {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--accent);
}

.row.selected { background: rgba(88, 166, 255, 0.08); }
.row.selected:hover { background: rgba(88, 166, 255, 0.14); }
.row.active { background: rgba(88, 166, 255, 0.1); outline: 1px solid rgba(88, 166, 255, 0.3); }
.row.active:hover { background: rgba(88, 166, 255, 0.16); }

/* Confirm delete modal */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  width: 480px;
  max-width: 90vw;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-title {
  padding: 16px 20px 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  border-bottom: 1px solid var(--border);
}

.modal-body {
  padding: 14px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 320px;
  overflow-y: auto;
}

.modal-warn {
  font-size: 12px;
  color: var(--orange, #e3b341);
  background: rgba(227, 179, 65, 0.1);
  border: 1px solid rgba(227, 179, 65, 0.25);
  border-radius: 6px;
  padding: 8px 10px;
}

.delete-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.delete-path {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.delete-more {
  font-size: 11px;
  color: var(--muted);
  opacity: 0.7;
  font-style: italic;
}

.delete-errors {
  border-top: 1px solid var(--border);
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.delete-errors-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--red, #da3633);
}

.delete-error {
  font-size: 11px;
  color: var(--muted);
  word-break: break-all;
}

.modal-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
