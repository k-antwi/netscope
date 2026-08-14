<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FileMatch, FileDetails, FileProcess, MalwareCheckResult } from '../types'

const props = defineProps<{
  file: FileMatch
  details: FileDetails | null
  isLoading: boolean
}>()

defineEmits<{ close: [] }>()

const copiedPath = ref(false)
let copyTimer: ReturnType<typeof setTimeout> | null = null

const isCheckingMalware = ref(false)
const malwareResult = ref<MalwareCheckResult | null>(null)
const malwareError = ref<string | null>(null)

watch(() => props.file.path, () => {
  isCheckingMalware.value = false
  malwareResult.value = null
  malwareError.value = null
})

async function checkMalware() {
  isCheckingMalware.value = true
  malwareError.value = null
  malwareResult.value = null
  try {
    malwareResult.value = await invoke<MalwareCheckResult>('check_malware', { path: props.file.path })
  } catch (e) {
    malwareError.value = String(e)
  } finally {
    isCheckingMalware.value = false
  }
}

const MALWARE_LABELS: Record<MalwareCheckResult['status'], string> = {
  clean: 'No threats detected',
  malicious: 'Malicious',
  suspicious: 'Suspicious',
  unknown: 'Not in database',
  no_api_key: 'Malware check unavailable',
  error: 'Check failed',
}

function malwareColor(status: MalwareCheckResult['status']): string {
  if (status === 'malicious') return 'orange'
  if (status === 'suspicious') return 'orange'
  if (status === 'clean') return 'green'
  return 'muted'
}

async function copyPath() {
  try {
    await navigator.clipboard.writeText(props.file.path)
  } catch {
    const el = document.createElement('textarea')
    el.value = props.file.path
    el.style.position = 'fixed'
    el.style.opacity = '0'
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    document.body.removeChild(el)
  }
  copiedPath.value = true
  if (copyTimer) clearTimeout(copyTimer)
  copyTimer = setTimeout(() => (copiedPath.value = false), 1400)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++ }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`
}

function formatDate(secs: number | null): string {
  if (!secs) return '—'
  return new Date(secs * 1000).toLocaleString(undefined, {
    year: 'numeric', month: 'short', day: '2-digit',
    hour: '2-digit', minute: '2-digit',
  })
}

function revealInFinder() {
  invoke('reveal_in_finder', { path: props.file.path }).catch(() => {})
}

function accessColor(access: string): string {
  if (access.includes('write')) return 'orange'
  if (access === 'read') return 'green'
  if (access === 'code segment') return 'blue'
  return 'muted'
}
</script>

<template>
  <aside class="detail">
    <!-- Header -->
    <div class="detail-header">
      <div class="header-left">
        <span class="file-icon">{{ file.is_dir ? '📁' : '📄' }}</span>
        <div class="detail-title" :title="file.path">{{ file.name }}</div>
      </div>
      <div class="header-actions">
        <button
          class="icon-btn"
          @click="revealInFinder"
          title="Show in Finder"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="4" width="9" height="9" rx="1.5"/>
            <path d="M8 2h6v6"/>
            <path d="M14 2L8 8"/>
          </svg>
        </button>
        <button class="close-btn" @click="$emit('close')">✕</button>
      </div>
    </div>

    <div class="detail-body">

      <!-- Info rows -->
      <div class="section">
        <div class="section-label">File Info</div>

        <div class="info-grid">
          <div class="info-row">
            <span class="info-key">Path</span>
            <span class="info-val path-val mono">
              <span class="path-text" :title="file.path">{{ file.path }}</span>
              <button
                class="copy-btn"
                :class="{ done: copiedPath }"
                @click="copyPath"
                :title="copiedPath ? 'Copied!' : 'Copy path'"
              >{{ copiedPath ? '✓' : '⧉' }}</button>
            </span>
          </div>

          <template v-if="details">
            <div class="info-row">
              <span class="info-key">Kind</span>
              <span class="info-val">{{ details.kind }}</span>
            </div>
            <div v-if="!details.is_dir" class="info-row">
              <span class="info-key">Size</span>
              <span class="info-val mono">{{ formatSize(details.size) }}</span>
            </div>
            <div class="info-row">
              <span class="info-key">Modified</span>
              <span class="info-val mono">{{ formatDate(details.modified) }}</span>
            </div>
            <div v-if="details.created" class="info-row">
              <span class="info-key">Created</span>
              <span class="info-val mono">{{ formatDate(details.created) }}</span>
            </div>
            <div class="info-row">
              <span class="info-key">Permissions</span>
              <span class="info-val mono perm">{{ details.permissions }}</span>
            </div>
          </template>

          <template v-else-if="isLoading">
            <div class="info-row">
              <span class="info-key">Kind</span>
              <span class="info-val skeleton" style="width: 90px" />
            </div>
            <div class="info-row">
              <span class="info-key">Size</span>
              <span class="info-val skeleton" style="width: 60px" />
            </div>
            <div class="info-row">
              <span class="info-key">Modified</span>
              <span class="info-val skeleton" style="width: 140px" />
            </div>
          </template>
        </div>
      </div>

      <div class="divider" />

      <!-- Processes -->
      <div class="section">
        <div class="section-label">Open by Processes</div>

        <div v-if="isLoading" class="proc-empty">
          <span class="spin">↺</span> checking…
        </div>

        <div
          v-else-if="!details || details.processes.length === 0"
          class="proc-empty"
        >
          No processes have this {{ file.is_dir ? 'directory' : 'file' }} open
        </div>

        <div v-else class="proc-list">
          <div
            v-for="proc in details.processes"
            :key="proc.pid"
            class="proc-row"
          >
            <div class="proc-main">
              <span class="proc-name">{{ proc.name }}</span>
              <span class="proc-pid mono">PID {{ proc.pid }}</span>
            </div>
            <span
              class="access-badge"
              :class="accessColor(proc.access)"
            >{{ proc.access }}</span>
          </div>
        </div>
      </div>

      <div v-if="!file.is_dir" class="divider" />

      <!-- Malware check -->
      <div v-if="!file.is_dir" class="section">
        <div class="section-label">Malware Check</div>

        <button
          v-if="!malwareResult && !malwareError"
          class="malware-btn"
          :disabled="isCheckingMalware"
          @click="checkMalware"
        >
          <span v-if="isCheckingMalware" class="spin">↺</span>
          <svg v-else width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M8 1.5l5.5 2v4c0 3.5-2.3 6-5.5 7-3.2-1-5.5-3.5-5.5-7v-4l5.5-2z"/>
            <path d="M5.7 8l1.7 1.7 3-3.2"/>
          </svg>
          {{ isCheckingMalware ? 'Scanning…' : 'Scan for Malware' }}
        </button>

        <div v-if="malwareError" class="malware-result">
          <span class="access-badge muted">Check failed</span>
          <span class="malware-msg">{{ malwareError }}</span>
          <button class="malware-retry" @click="checkMalware">Retry</button>
        </div>

        <div v-else-if="malwareResult" class="malware-result">
          <span class="access-badge" :class="malwareColor(malwareResult.status)">
            {{ MALWARE_LABELS[malwareResult.status] }}
          </span>
          <span
            v-if="malwareResult.status === 'malicious' || malwareResult.status === 'suspicious'"
            class="malware-msg"
          >{{ malwareResult.malicious + malwareResult.suspicious }} / {{ malwareResult.total_engines }} engines flagged this file</span>
          <span v-else-if="malwareResult.status === 'clean'" class="malware-msg">
            0 / {{ malwareResult.total_engines }} engines flagged this file
          </span>
          <span v-else-if="malwareResult.message" class="malware-msg">{{ malwareResult.message }}</span>

          <a
            v-if="malwareResult.permalink"
            :href="malwareResult.permalink"
            target="_blank"
            rel="noopener noreferrer"
            class="malware-link"
          >View on VirusTotal ↗</a>

          <div v-if="malwareResult.malware_bazaar" class="mb-row">
            <span class="malware-msg">MalwareBazaar:</span>
            <span
              class="access-badge"
              :class="malwareResult.malware_bazaar.found ? 'orange' : 'green'"
            >{{ malwareResult.malware_bazaar.found
              ? (malwareResult.malware_bazaar.signature || 'Known malware sample')
              : 'Not found' }}</span>
            <a
              v-if="malwareResult.malware_bazaar.found"
              :href="`https://bazaar.abuse.ch/sample/${malwareResult.sha256}/`"
              target="_blank"
              rel="noopener noreferrer"
              class="malware-link"
            >View ↗</a>
          </div>

          <button v-if="malwareResult.status !== 'no_api_key'" class="malware-retry" @click="checkMalware">Re-scan</button>
        </div>
      </div>

    </div>
  </aside>
</template>

<style scoped>
.detail {
  width: var(--panel-w);
  flex-shrink: 0;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  gap: 10px;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
}

.file-icon { font-size: 18px; flex-shrink: 0; margin-top: 1px; }

.detail-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  line-height: 1.4;
  word-break: break-all;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
  margin-top: 1px;
}

.icon-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 4px 5px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover { color: var(--accent); background: var(--surface-2); }

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 14px;
  padding: 4px;
  border-radius: 4px;
  flex-shrink: 0;
}
.close-btn:hover { color: var(--text); background: var(--surface-2); }

.detail-body {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 14px 16px;
}

.section { display: flex; flex-direction: column; gap: 10px; }

.section-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.divider { height: 1px; background: var(--border); }

/* Info grid */
.info-grid { display: flex; flex-direction: column; gap: 0; }

.info-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid rgba(48, 54, 61, 0.4);
  font-size: 12px;
}
.info-row:last-child { border-bottom: none; }

.info-key {
  flex-shrink: 0;
  width: 82px;
  color: var(--muted);
  padding-top: 1px;
}

.info-val {
  flex: 1;
  color: var(--text);
  min-width: 0;
  word-break: break-all;
  line-height: 1.5;
}

.path-val {
  display: flex;
  align-items: flex-start;
  gap: 6px;
}

.path-text {
  flex: 1;
  min-width: 0;
  word-break: break-all;
  line-height: 1.5;
  color: var(--muted);
  font-size: 11px;
}

.perm { letter-spacing: 0.04em; color: var(--muted); }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }

.copy-btn {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
  padding: 1px 3px;
  border-radius: 3px;
  line-height: 1;
  margin-top: 2px;
}
.copy-btn:hover { color: var(--accent); background: var(--surface-2); }
.copy-btn.done { color: var(--green); }

/* Skeleton shimmer */
@keyframes shimmer {
  from { opacity: 0.35; }
  50%  { opacity: 0.7;  }
  to   { opacity: 0.35; }
}
.skeleton {
  display: inline-block;
  height: 12px;
  border-radius: 4px;
  background: var(--surface-2);
  animation: shimmer 1.4s ease-in-out infinite;
}

/* Processes */
.proc-empty {
  font-size: 12px;
  color: var(--muted);
  padding: 10px 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

@keyframes spin { to { transform: rotate(360deg); } }
.spin { display: inline-block; animation: spin 1s linear infinite; }

.proc-list { display: flex; flex-direction: column; gap: 6px; }

.proc-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 7px;
}

.proc-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.proc-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.proc-pid {
  font-size: 10px;
  color: var(--muted);
}

.access-badge {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 2px 7px;
  border-radius: 4px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--muted);
}
.access-badge.orange { color: var(--orange); border-color: rgba(210,153,34,0.3); background: rgba(210,153,34,0.1); }
.access-badge.green  { color: var(--green);  border-color: rgba(63,185,80,0.3);  background: var(--green-dim); }
.access-badge.blue   { color: var(--accent); border-color: rgba(88,166,255,0.25); background: rgba(88,166,255,0.07); }
.access-badge.muted  { color: var(--muted); }

/* Malware check */
.malware-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  align-self: flex-start;
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 7px;
  padding: 7px 12px;
  cursor: pointer;
}
.malware-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.malware-btn:disabled { cursor: default; opacity: 0.7; }

.malware-result {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.malware-msg {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.5;
}

.mb-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.malware-link {
  font-size: 11px;
  color: var(--accent);
  text-decoration: none;
}
.malware-link:hover { text-decoration: underline; }

.malware-retry {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 11px;
  cursor: pointer;
  padding: 0;
  text-decoration: underline;
}
.malware-retry:hover { color: var(--accent); }
</style>
