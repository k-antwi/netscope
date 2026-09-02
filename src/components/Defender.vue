<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ThreatItem, DefenderProgress, DefenderScanResult, StoredScan, SecurityReport, IntruderFinding, IntruderReport } from '../types'
import IntruderDetail from './IntruderDetail.vue'
import ThreatDetail from './ThreatDetail.vue'

type ScanType = 'full' | 'quick' | 'custom'
type DefenderPane = 'scanner' | 'reports' | 'intruder'

const isScanning = ref(false)
const activeScanType = ref<ScanType | null>(null)
const progress = ref<DefenderProgress | null>(null)
const result = ref<DefenderScanResult | null>(null)
const error = ref('')
const customPathsInput = ref('')
const customDialogOpen = ref(false)
const neutralizing = ref<Set<string>>(new Set())
const neutralized = ref<Set<string>>(new Set())
const neutralizeErrors = ref<Map<string, string>>(new Map())

// Persistence — survives tab switches AND app restarts
const storedTimestamp = ref<number | null>(null)
const storedUnfixedCount = ref(0)
const lastStoredResult = ref<DefenderScanResult | null>(null)

// Full Disk Access
const hasFullDiskAccess = ref<boolean | null>(null)
const fdaDismissed = ref(false)

// Reports pane
const activePane = ref<DefenderPane>('scanner')
const reports = ref<SecurityReport[]>([])
const reportsLoading = ref(false)
const expandedReport = ref<string | null>(null)

// Threat detail flyover
const selectedThreat = ref<ThreatItem | null>(null)
const selectedThreatReport = ref<SecurityReport | null>(null)
const selectedThreatNeutralized = computed(() => {
  if (!selectedThreat.value) return false
  if (selectedThreatReport.value) {
    return selectedThreatReport.value.neutralized.includes(selectedThreat.value.path)
  }
  return neutralized.value.has(selectedThreat.value.path)
})

function openThreatDetail(t: ThreatItem, report: SecurityReport | null = null) {
  selectedThreat.value = t
  selectedThreatReport.value = report
}

function onThreatNeutralized(path: string) {
  neutralized.value = new Set([...neutralized.value, path])
  storedUnfixedCount.value = Math.max(0, storedUnfixedCount.value - 1)
  persistScan()
}

// Intruder pane
const intruderRunning = ref(false)
const intruderReport = ref<IntruderReport | null>(null)
const intruderError = ref('')
const selectedFinding = ref<IntruderFinding | null>(null)

let unlisten: UnlistenFn | null = null

const threats = computed(() => result.value?.threats ?? [])
const activeThreats = computed(() => threats.value.filter(t => !neutralized.value.has(t.path)))

function daysAgo(unixSecs: number): string {
  const days = Math.floor((Date.now() / 1000 - unixSecs) / 86400)
  if (days === 0) return 'today'
  if (days === 1) return '1 day ago'
  return `${days} days ago`
}

function lastScanText(): string {
  if (!storedTimestamp.value) return 'Never scanned'
  return `Last scanned ${daysAgo(storedTimestamp.value)}`
}

async function persistScan() {
  if (!result.value) return
  await invoke('save_defender_scan', {
    result: result.value,
    neutralized: [...neutralized.value],
  }).catch(() => {})
}

async function loadReports() {
  reportsLoading.value = true
  try {
    reports.value = await invoke<SecurityReport[]>('load_security_reports')
  } catch { /* ignore */ } finally {
    reportsLoading.value = false
  }
}

async function switchPane(pane: DefenderPane) {
  activePane.value = pane
  selectedThreat.value = null
  selectedThreatReport.value = null
  if (pane === 'reports') await loadReports()
}

async function runIntruderScan() {
  if (intruderRunning.value) return
  intruderRunning.value = true
  intruderError.value = ''
  intruderReport.value = null
  selectedFinding.value = null
  try {
    intruderReport.value = await invoke<IntruderReport>('spot_intruder')
  } catch (e) {
    intruderError.value = typeof e === 'string' ? e : String(e)
  } finally {
    intruderRunning.value = false
  }
}

function onFindingClick(f: IntruderFinding) {
  selectedFinding.value = selectedFinding.value?.id === f.id ? null : f
}

function categoryLabel(c: string): string {
  const labels: Record<string, string> = {
    known_bad_port:    'Malicious Port',
    suspicious_process:'Suspicious Process',
    lateral_movement:  'Lateral Movement',
    port_scan:         'Port Scan',
    backdoor_listener: 'Backdoor',
    cleartext_exfil:   'Cleartext Exfil',
  }
  return labels[c] ?? c
}

function intruderSeverityCount(sev: string): number {
  return intruderReport.value?.findings.filter((f: IntruderFinding) => f.severity === sev).length ?? 0
}

async function refreshFdaStatus() {
  hasFullDiskAccess.value = await invoke<boolean>('check_full_disk_access').catch(() => false)
}

async function openFdaSettings() {
  await invoke('open_full_disk_access_settings').catch(() => {})
  // Re-check after a short delay so the banner updates if the user grants access
  setTimeout(refreshFdaStatus, 3000)
  setTimeout(refreshFdaStatus, 8000)
}

onMounted(async () => {
  try {
    const stored = await invoke<StoredScan | null>('load_last_defender_scan')
    if (stored) {
      // Only restore metadata — don't set result.value so Defender always opens on Scanner
      lastStoredResult.value = stored.result
      neutralized.value = new Set(stored.neutralized)
      storedTimestamp.value = stored.timestamp
      storedUnfixedCount.value = stored.result.threats.filter(
        (t: ThreatItem) => !stored.neutralized.includes(t.path)
      ).length
    }
  } catch { /* first launch or corrupted file — silently ignore */ }

  await refreshFdaStatus()
})

function checkNow() {
  result.value = lastStoredResult.value
  switchPane('reports')
}

async function startScan(type: ScanType, paths: string[] = []) {
  if (isScanning.value) return

  activeScanType.value = type
  isScanning.value = true
  error.value = ''
  result.value = null
  progress.value = { scanned_files: 0, threats_found: 0, current: '' }
  neutralized.value = new Set()
  neutralizing.value = new Set()
  neutralizeErrors.value = new Map()

  // Pre-warm macOS TCC permissions for all protected directories so the
  // system dialogs appear before the scan starts, not mid-run.
  if (type === 'full' || type === 'quick') {
    await invoke('request_scan_permissions').catch(() => {})
  }

  if (unlisten) { unlisten(); unlisten = null }
  unlisten = await listen<DefenderProgress>('defender-progress', e => {
    progress.value = e.payload
  })

  try {
    result.value = await invoke<DefenderScanResult>('scan_for_threats', {
      scanType: type,
      customPaths: paths,
    })
    if (result.value) {
      lastStoredResult.value = result.value
      storedTimestamp.value = Math.floor(Date.now() / 1000)
      storedUnfixedCount.value = result.value.threats.length
      await persistScan()
      await invoke('save_security_report', {
        result: result.value,
        neutralized: [],
      }).catch(() => {})
      await switchPane('reports')
    }
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e)
  } finally {
    isScanning.value = false
    progress.value = null
    if (unlisten) { unlisten(); unlisten = null }
  }
}

function initiateScan(type: ScanType) {
  if (type === 'custom') {
    customDialogOpen.value = true
    return
  }
  startScan(type)
}

function submitCustomPaths() {
  const paths = customPathsInput.value
    .split('\n')
    .map(p => p.trim())
    .filter(p => p.length > 0)
  if (paths.length === 0) return
  customDialogOpen.value = false
  startScan('custom', paths)
}

async function stopScan() {
  await invoke('cancel_defender_scan').catch(() => {})
}

async function neutralize(threat: ThreatItem) {
  neutralizing.value = new Set([...neutralizing.value, threat.path])
  try {
    await invoke('neutralize_threat', { path: threat.path })
    neutralized.value = new Set([...neutralized.value, threat.path])
    const errs = new Map(neutralizeErrors.value)
    errs.delete(threat.path)
    neutralizeErrors.value = errs
    storedUnfixedCount.value = Math.max(0, storedUnfixedCount.value - 1)
    await persistScan()
  } catch (e) {
    const errs = new Map(neutralizeErrors.value)
    errs.set(threat.path, typeof e === 'string' ? e : String(e))
    neutralizeErrors.value = errs
  } finally {
    const next = new Set(neutralizing.value)
    next.delete(threat.path)
    neutralizing.value = next
  }
}

function severityLabel(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}

function threatTypeLabel(t: string): string {
  const labels: Record<string, string> = {
    suspicious_launchagent: 'Launch Agent',
    suspicious_script: 'Script',
    suspicious_executable: 'Executable',
    hidden_executable: 'Hidden File',
  }
  return labels[t] ?? t
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let v = bytes / 1024, u = 0
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[u]}`
}

function formatElapsed(ms: number): string {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`
}

function truncate(path: string, max = 56): string {
  return path.length <= max ? path : `…${path.slice(path.length - max + 1)}`
}

function formatReportDate(unixSecs: number): string {
  const d = new Date(unixSecs * 1000)
  const now = new Date()
  const diffDays = Math.floor((now.getTime() - d.getTime()) / 86400000)
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  if (diffDays === 0) return `Today, ${time}`
  if (diffDays === 1) return `Yesterday, ${time}`
  return `${d.toLocaleDateString([], { month: 'short', day: 'numeric' })}, ${time}`
}

function reportStatusClass(report: SecurityReport): string {
  const active = report.result.threats.filter(t => !report.neutralized.includes(t.path)).length
  if (active > 0) return 'status-danger'
  if (report.result.threats.length > 0) return 'status-ok'
  return 'status-clean'
}

function toggleReport(id: string) {
  expandedReport.value = expandedReport.value === id ? null : id
}

onUnmounted(() => {
  if (unlisten) unlisten()
  if (isScanning.value) invoke('cancel_defender_scan').catch(() => {})
})
</script>

<template>
  <div class="defender">

    <!-- ── Scanning state ── -->
    <div v-if="isScanning" class="center-state">
      <span class="spin-lg">↺</span>
      <div class="state-title">
        Scanning for threats
        <span class="scan-type-badge">{{ activeScanType }}</span>
      </div>
      <div class="progress-stats">
        <span><b>{{ (progress?.scanned_files ?? 0).toLocaleString() }}</b> files scanned</span>
        <span class="sep">·</span>
        <span><b>{{ progress?.threats_found ?? 0 }}</b> threats found</span>
      </div>
      <div class="progress-path mono">{{ truncate(progress?.current || '') || '…' }}</div>
      <button class="btn ghost" @click="stopScan">Stop scan</button>
    </div>

    <!-- ── Error state ── -->
    <div v-else-if="error" class="center-state">
      <span class="state-icon error">⚠</span>
      <div class="state-title">Scan failed</div>
      <div class="state-sub">{{ error }}</div>
      <button class="btn primary" @click="result = null; error = ''">Back</button>
    </div>

    <!-- ── Idle / scan selection ── -->
    <template v-else>
      <!-- Pane switcher -->
      <div class="pane-nav">
        <button
          class="pane-tab"
          :class="{ active: activePane === 'scanner' }"
          @click="switchPane('scanner')"
        >Scanner</button>
        <button
          class="pane-tab"
          :class="{ active: activePane === 'reports' }"
          @click="switchPane('reports')"
        >Reports</button>
        <button
          class="pane-tab"
          :class="{ active: activePane === 'intruder' }"
          @click="switchPane('intruder')"
        >
          Intruder
          <span
            v-if="intruderReport && intruderReport.findings.length > 0"
            class="pane-badge"
          >{{ intruderReport.findings.length }}</span>
        </button>
      </div>

      <!-- Intruder pane -->
      <div v-if="activePane === 'intruder'" class="intruder-pane">

        <!-- Running -->
        <div v-if="intruderRunning" class="center-state">
          <div class="radar-running">
            <svg viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="60" cy="60" r="54" stroke="rgba(88,166,255,0.18)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="38" stroke="rgba(88,166,255,0.14)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="22" stroke="rgba(88,166,255,0.18)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="6" fill="rgba(88,166,255,0.35)"/>
              <line x1="60" y1="6" x2="60" y2="114" stroke="rgba(88,166,255,0.07)" stroke-width="1"/>
              <line x1="6" y1="60" x2="114" y2="60" stroke="rgba(88,166,255,0.07)" stroke-width="1"/>
              <path d="M60 60 L60 6 A54 54 0 0 1 113 77 Z" fill="url(#sweep-grad)" class="radar-sweep-arm"/>
            </svg>
            <defs>
              <radialGradient id="sweep-grad" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(60 60) scale(54)">
                <stop offset="0%" stop-color="rgba(88,166,255,0.35)"/>
                <stop offset="100%" stop-color="rgba(88,166,255,0)"/>
              </radialGradient>
            </defs>
          </div>
          <div class="state-title">Analyzing network activity</div>
          <div class="state-sub">Scanning connections for intrusion patterns…</div>
        </div>

        <!-- Error -->
        <div v-else-if="intruderError" class="center-state">
          <span class="state-icon error">⚠</span>
          <div class="state-title">Analysis failed</div>
          <div class="state-sub">{{ intruderError }}</div>
          <button class="btn ghost" @click="intruderError = ''">Back</button>
        </div>

        <!-- Findings -->
        <template v-else-if="intruderReport && intruderReport.findings.length > 0">
          <div class="intruder-results-bar">
            <div class="irb-left">
              <span class="irb-count"><b>{{ intruderReport.findings.length }}</b> {{ intruderReport.findings.length === 1 ? 'finding' : 'findings' }}</span>
              <span class="sep">·</span>
              <span class="irb-stat">{{ intruderReport.connections_analyzed }} connections analyzed</span>
              <span v-if="intruderSeverityCount('critical') > 0" class="sev-pill critical">{{ intruderSeverityCount('critical') }} critical</span>
              <span v-if="intruderSeverityCount('high') > 0" class="sev-pill high">{{ intruderSeverityCount('high') }} high</span>
              <span v-if="intruderSeverityCount('medium') > 0" class="sev-pill medium">{{ intruderSeverityCount('medium') }} medium</span>
            </div>
            <button class="btn ghost" @click="runIntruderScan">Re-run</button>
          </div>

          <div class="intruder-findings-content">
            <div class="finding-list">
              <div
                v-for="f in intruderReport.findings"
                :key="f.id"
                class="finding-card"
                :class="[f.severity, { selected: selectedFinding?.id === f.id }]"
                @click="onFindingClick(f)"
              >
                <div class="fc-top">
                  <span class="sev-badge" :class="f.severity">{{ severityLabel(f.severity) }}</span>
                  <span class="cat-tag">{{ categoryLabel(f.category) }}</span>
                  <span class="fc-proc">
                    {{ f.process }}
                    <span class="fc-pid">PID {{ f.pid }}</span>
                  </span>
                </div>
                <div class="fc-title">{{ f.title }}</div>
                <div class="fc-desc">{{ f.description }}</div>
              </div>
            </div>

            <Transition name="panel">
              <IntruderDetail
                v-if="selectedFinding"
                :finding="selectedFinding"
                @close="selectedFinding = null"
              />
            </Transition>
          </div>
        </template>

        <!-- Clean -->
        <div v-else-if="intruderReport && intruderReport.findings.length === 0" class="center-state grow">
          <span class="clean-shield">
            <svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M32 4L8 14v18c0 16 11 28 24 30 13-2 24-14 24-30V14L32 4z" stroke="var(--green)" stroke-width="2.5"/>
              <polyline points="22 32 29 39 42 25" stroke="var(--green)" stroke-width="3"/>
            </svg>
          </span>
          <div class="state-title">No intrusions detected</div>
          <div class="state-sub">
            Analyzed {{ intruderReport.connections_analyzed }} active connections — no suspicious activity found.
          </div>
          <button class="btn ghost" style="margin-top:4px" @click="runIntruderScan">Re-run analysis</button>
        </div>

        <!-- Idle -->
        <div v-else class="intruder-hero">
          <div class="radar-idle">
            <svg viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="60" cy="60" r="54" stroke="rgba(88,166,255,0.15)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="38" stroke="rgba(88,166,255,0.12)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="22" stroke="rgba(88,166,255,0.18)" stroke-width="1.5"/>
              <circle cx="60" cy="60" r="6" fill="rgba(88,166,255,0.3)" stroke="rgba(88,166,255,0.5)" stroke-width="1"/>
              <line x1="60" y1="6" x2="60" y2="114" stroke="rgba(88,166,255,0.07)" stroke-width="1"/>
              <line x1="6" y1="60" x2="114" y2="60" stroke="rgba(88,166,255,0.07)" stroke-width="1"/>
            </svg>
          </div>
          <div class="intruder-title">Spot the Intruder</div>
          <div class="intruder-desc">
            Analyzes live network connections using cybersecurity heuristics — detecting backdoors, C2 beacons, lateral movement, suspicious processes, and malicious port usage.
          </div>
          <div class="intruder-techniques">
            <span class="technique-tag">Malicious ports</span>
            <span class="technique-tag">Lateral movement</span>
            <span class="technique-tag">Backdoor listeners</span>
            <span class="technique-tag">Suspicious processes</span>
            <span class="technique-tag">Port scan detection</span>
            <span class="technique-tag">Cleartext exfiltration</span>
          </div>
          <button class="btn intruder-run-btn" @click="runIntruderScan">
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:16px;height:16px">
              <circle cx="10" cy="10" r="8.5"/>
              <circle cx="10" cy="10" r="5" stroke-dasharray="3 2"/>
              <circle cx="10" cy="10" r="1.5" fill="currentColor" stroke="none"/>
            </svg>
            Run Analysis
          </button>
        </div>
      </div>

      <!-- Reports pane -->
      <div v-else-if="activePane === 'reports'" class="reports-pane">
      <div class="reports-main">

        <!-- Current scan results (shown immediately after a scan runs) -->
        <template v-if="result">
          <div class="rescan-bar">
            <div class="rescan-info">
              <span class="rescan-stat">
                <b>{{ result.threats.length }}</b>
                {{ result.threats.length === 1 ? 'threat' : 'threats' }} found
              </span>
              <span class="sep">·</span>
              <span class="rescan-stat">{{ result.scanned_files.toLocaleString() }} files scanned</span>
              <span class="sep">·</span>
              <span class="rescan-stat">{{ formatElapsed(result.elapsed_ms) }}</span>
              <span v-if="result.cancelled" class="tag warn">stopped early</span>
            </div>
            <div class="rescan-actions">
              <button class="btn ghost" @click="initiateScan('quick')">Quick Scan</button>
              <button class="btn ghost" @click="initiateScan('full')">Full Scan</button>
              <button class="btn ghost" @click="result = null; switchPane('scanner')">New Scan</button>
            </div>
          </div>

          <!-- Clean result -->
          <div v-if="threats.length === 0" class="center-state grow">
            <span class="clean-shield">
              <svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M32 4L8 14v18c0 16 11 28 24 30 13-2 24-14 24-30V14L32 4z" stroke="var(--green)" stroke-width="2.5"/>
                <polyline points="22 32 29 39 42 25" stroke="var(--green)" stroke-width="3"/>
              </svg>
            </span>
            <div class="state-title">System clean</div>
            <div class="state-sub">No threats found across {{ result.scanned_files.toLocaleString() }} files.</div>
          </div>

          <!-- Threat list -->
          <div v-else class="threat-list-wrap">
            <div v-if="activeThreats.length === 0 && neutralized.size > 0" class="center-state grow">
              <span class="clean-shield">
                <svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M32 4L8 14v18c0 16 11 28 24 30 13-2 24-14 24-30V14L32 4z" stroke="var(--green)" stroke-width="2.5"/>
                  <polyline points="22 32 29 39 42 25" stroke="var(--green)" stroke-width="3"/>
                </svg>
              </span>
              <div class="state-title">All threats neutralized</div>
              <div class="state-sub">{{ neutralized.size }} item{{ neutralized.size === 1 ? '' : 's' }} moved to quarantine.</div>
            </div>

            <table v-else>
              <thead>
                <tr>
                  <th>Threat</th>
                  <th>Type</th>
                  <th>Severity</th>
                  <th>Location</th>
                  <th>Size</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="t in threats"
                  :key="t.path"
                  :class="['threat-row', { neutralized: neutralized.has(t.path), selected: selectedThreat?.path === t.path }]"
                  @click="openThreatDetail(t)"
                >
                  <td class="threat-name">
                    <span class="tname">{{ t.name }}</span>
                    <span class="treason">{{ t.reason }}</span>
                  </td>
                  <td><span class="type-tag">{{ threatTypeLabel(t.threat_type) }}</span></td>
                  <td>
                    <span class="sev-badge" :class="t.severity">{{ severityLabel(t.severity) }}</span>
                  </td>
                  <td class="path-cell mono">{{ truncate(t.path) }}</td>
                  <td class="size-cell mono">{{ formatSize(t.size) }}</td>
                  <td class="action-cell">
                    <span v-if="neutralized.has(t.path)" class="neutralized-label">Quarantined</span>
                    <div v-else class="action-group">
                      <div v-if="neutralizeErrors.get(t.path)" class="action-error">
                        {{ neutralizeErrors.get(t.path) }}
                      </div>
                      <button
                        class="btn danger"
                        :disabled="neutralizing.has(t.path)"
                        @click.stop="neutralize(t)"
                      >
                        {{ neutralizing.has(t.path) ? '…' : 'Neutralize' }}
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <!-- Historical reports (shown when no active scan result) -->
        <template v-else>
        <div v-if="reportsLoading" class="center-state">
          <span class="spin-lg">↺</span>
          <span>Loading reports…</span>
        </div>
        <div v-else-if="reports.length === 0" class="center-state">
          <svg viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="width:44px;height:44px;opacity:.35">
            <rect x="8" y="4" width="32" height="40" rx="4"/>
            <line x1="16" y1="16" x2="32" y2="16"/>
            <line x1="16" y1="23" x2="32" y2="23"/>
            <line x1="16" y1="30" x2="24" y2="30"/>
          </svg>
          <div style="font-size:13px;color:var(--text)">No reports yet</div>
          <div class="state-sub">Reports are saved automatically after each scan and kept for 5 days.</div>
        </div>
        <div v-else class="report-list">
          <div
            v-for="report in reports"
            :key="report.id"
            class="report-card"
            :class="{ expanded: expandedReport === report.id }"
          >
            <button class="report-summary" @click="toggleReport(report.id)">
              <span :class="['report-dot', reportStatusClass(report)]"></span>
              <span class="report-date">{{ formatReportDate(report.timestamp) }}</span>
              <span class="report-type-badge">{{ report.result.scan_type }}</span>
              <span class="report-stat">
                <b>{{ report.result.scanned_files.toLocaleString() }}</b> files
              </span>
              <span class="sep">·</span>
              <span class="report-stat">
                <b
                  :style="report.result.threats.length > 0 ? 'color:var(--red)' : ''"
                >{{ report.result.threats.length }}</b>
                {{ report.result.threats.length === 1 ? 'threat' : 'threats' }}
              </span>
              <span v-if="report.neutralized.length > 0" class="sep">·</span>
              <span v-if="report.neutralized.length > 0" class="report-stat neutralized-stat">
                {{ report.neutralized.length }} quarantined
              </span>
              <span v-if="report.result.cancelled" class="tag warn" style="margin-left:4px">stopped</span>
              <span class="report-elapsed">{{ formatElapsed(report.result.elapsed_ms) }}</span>
              <span class="report-chevron" :class="{ open: expandedReport === report.id }">›</span>
            </button>

            <div v-if="expandedReport === report.id" class="report-detail">
              <div v-if="report.result.threats.length === 0" class="report-clean">
                <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:16px;height:16px;flex-shrink:0">
                  <circle cx="10" cy="10" r="8.5" stroke="var(--green)"/>
                  <polyline points="6.5 10 8.5 12.5 13.5 7.5" stroke="var(--green)"/>
                </svg>
                <span>No threats found — system was clean.</span>
              </div>
              <table v-else class="report-table">
                <thead>
                  <tr>
                    <th>Threat</th>
                    <th>Type</th>
                    <th>Severity</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="t in report.result.threats"
                    :key="t.path"
                    :class="['threat-row', { selected: selectedThreat?.path === t.path }]"
                    @click="openThreatDetail(t, report)"
                  >
                    <td class="threat-name">
                      <span class="tname">{{ t.name }}</span>
                      <span class="treason">{{ t.reason }}</span>
                    </td>
                    <td><span class="type-tag">{{ threatTypeLabel(t.threat_type) }}</span></td>
                    <td><span class="sev-badge" :class="t.severity">{{ severityLabel(t.severity) }}</span></td>
                    <td>
                      <span v-if="report.neutralized.includes(t.path)" class="neutralized-label">Quarantined</span>
                      <span v-else class="active-threat-label">Active</span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
        </template>
      </div><!-- reports-main -->

      <Transition name="panel">
        <ThreatDetail
          v-if="selectedThreat"
          :threat="selectedThreat"
          :is-neutralized="selectedThreatNeutralized"
          @close="selectedThreat = null; selectedThreatReport = null"
          @neutralized="onThreatNeutralized"
        />
      </Transition>
      </div><!-- reports-pane -->

      <div v-else class="hero">
        <div class="hero-meta">
          <svg class="shield-bg" viewBox="0 0 200 220" fill="none">
            <path d="M100 8L12 42v64c0 52 37 95 88 106 51-11 88-54 88-106V42L100 8z"
              fill="rgba(88,166,255,0.06)" stroke="rgba(88,166,255,0.15)" stroke-width="1.5"/>
          </svg>
          <span class="last-scan-label">{{ lastScanText() }}</span>
        </div>

        <!-- Full Disk Access banner -->
        <Transition name="fda-banner">
          <div
            v-if="hasFullDiskAccess === false && !fdaDismissed"
            class="fda-banner"
          >
            <div class="fda-icon">
              <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="10" cy="10" r="8.5" stroke="var(--orange)"/>
                <line x1="10" y1="6.5" x2="10" y2="11" stroke="var(--orange)"/>
                <circle cx="10" cy="13.5" r="0.8" fill="var(--orange)" stroke="none"/>
              </svg>
            </div>
            <div class="fda-body">
              <div class="fda-title">Full Disk Access not granted</div>
              <div class="fda-desc">
                Without Full Disk Access, scans skip protected folders like
                Documents and Desktop, and macOS will prompt separately for each
                directory. Grant access once in System Settings for complete,
                uninterrupted scans.
              </div>
            </div>
            <div class="fda-actions">
              <button class="btn primary fda-btn" @click="openFdaSettings">
                Open System Settings
              </button>
              <button class="btn ghost fda-dismiss" @click="fdaDismissed = true" title="Dismiss">✕</button>
            </div>
          </div>
        </Transition>

        <div class="scan-cards">
          <!-- Full Scan -->
          <button class="scan-card" @click="initiateScan('full')">
            <div class="card-icon-wrap accent-blue">
              <svg class="card-icon" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="4" y="6" width="40" height="27" rx="3"/>
                <line x1="15" y1="41" x2="33" y2="41"/>
                <line x1="24" y1="33" x2="24" y2="41"/>
                <circle cx="24" cy="19" r="8"/>
                <line x1="24" y1="11" x2="24" y2="19"/>
                <circle cx="24" cy="19" r="2.5" fill="currentColor" stroke="none"/>
              </svg>
            </div>
            <span class="card-label">Full Scan</span>
            <span class="card-desc">Scans your entire home directory and system launch agents</span>
          </button>

          <!-- Quick Scan -->
          <button class="scan-card" @click="initiateScan('quick')">
            <div class="card-icon-wrap accent-teal">
              <svg class="card-icon" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="24" cy="24" r="18"/>
                <circle cx="24" cy="24" r="11" stroke-dasharray="4 3"/>
                <circle cx="24" cy="24" r="5"/>
                <circle cx="24" cy="24" r="1.5" fill="currentColor" stroke="none"/>
                <line x1="24" y1="6" x2="24" y2="19"/>
                <line x1="7" y1="24" x2="13" y2="24"/>
              </svg>
            </div>
            <span class="card-label">Quick Scan</span>
            <span class="card-desc">Checks Downloads, launch agents, and temp directories</span>
          </button>

          <!-- Custom Scan -->
          <button class="scan-card" @click="initiateScan('custom')">
            <div class="card-icon-wrap accent-purple">
              <svg class="card-icon" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 12a4 4 0 0 1 4-4h10l4 5h18a4 4 0 0 1 4 4v19a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4V12z"/>
                <circle cx="24" cy="28" r="6"/>
                <line x1="28.2" y1="32.2" x2="32" y2="36"/>
              </svg>
            </div>
            <span class="card-label">Custom Scan</span>
            <span class="card-desc">Select specific folders to scan for threats</span>
          </button>
        </div>
      </div>

      <!-- Previous scan status bar (scanner pane only) -->
      <template v-if="activePane === 'scanner'">
        <div v-if="storedTimestamp && storedUnfixedCount > 0" class="issue-bar warn">
          <svg class="issue-icon" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="10" cy="10" r="8.5" stroke="var(--red)"/>
            <line x1="10" y1="6" x2="10" y2="11" stroke="var(--red)"/>
            <circle cx="10" cy="14" r="0.8" fill="var(--red)" stroke="none"/>
          </svg>
          <span class="issue-text">
            <b>{{ storedUnfixedCount }} unfixed {{ storedUnfixedCount === 1 ? 'threat' : 'threats' }}</b>
            from your last scan that could cause harm
          </span>
          <button
            class="btn primary"
            style="margin-left: auto; flex-shrink: 0"
            @click="checkNow"
          >View in Reports</button>
        </div>
        <div v-else-if="storedTimestamp && storedUnfixedCount === 0" class="issue-bar clean">
          <svg class="issue-icon" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="10" cy="10" r="8.5" stroke="var(--green)"/>
            <polyline points="6.5 10 8.5 12.5 13.5 7.5" stroke="var(--green)"/>
          </svg>
          <span class="issue-text">Last scan was clean — {{ daysAgo(storedTimestamp) }}</span>
        </div>
      </template>
    </template>

    <!-- ── Custom path dialog ── -->
    <div v-if="customDialogOpen" class="modal-overlay" @click.self="customDialogOpen = false">
      <div class="modal">
        <div class="modal-title">Custom Scan — Select Directories</div>
        <div class="modal-body">
          <div class="modal-hint">Enter one directory path per line. Absolute paths only.</div>
          <textarea
            v-model="customPathsInput"
            class="path-textarea mono"
            placeholder="/Users/you/Documents&#10;/Applications"
            rows="5"
            spellcheck="false"
            autofocus
          />
        </div>
        <div class="modal-footer">
          <button class="btn ghost" @click="customDialogOpen = false">Cancel</button>
          <button class="btn primary" @click="submitCustomPaths">Start Scan</button>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.defender {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg);
}

/* ── Center states ── */
.center-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px;
  text-align: center;
  color: var(--muted);
}
.center-state.grow { flex: 1; }

.state-icon { font-size: 36px; line-height: 1; }
.state-icon.error { color: var(--red); }
.state-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 8px;
}
.state-sub { font-size: 12px; max-width: 400px; line-height: 1.6; }

@keyframes spin { to { transform: rotate(360deg); } }
.spin-lg { display: inline-block; animation: spin 1.1s linear infinite; font-size: 28px; }

.scan-type-badge {
  text-transform: capitalize;
  font-size: 11px;
  font-weight: 500;
  background: rgba(88, 166, 255, 0.12);
  color: var(--accent);
  border: 1px solid rgba(88, 166, 255, 0.25);
  padding: 2px 8px;
  border-radius: 10px;
}

.progress-stats { display: flex; gap: 6px; font-size: 12px; }
.progress-stats b { color: var(--text); font-variant-numeric: tabular-nums; }
.sep { opacity: 0.4; }
.progress-path {
  font-size: 11px;
  max-width: 600px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  opacity: 0.6;
}

/* ── Idle / hero ── */
.hero {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 40px;
  padding: 48px 40px 56px;
  background: radial-gradient(ellipse 80% 60% at 50% 0%, rgba(88, 166, 255, 0.10) 0%, transparent 70%);
  position: relative;
  overflow: hidden;
}

.hero-meta {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  position: relative;
}

.shield-bg {
  width: 200px;
  height: 220px;
  position: absolute;
  top: -60px;
  opacity: 0.6;
  pointer-events: none;
}

.last-scan-label {
  font-size: 12px;
  color: var(--muted);
  position: relative;
  z-index: 1;
}

/* ── Scan cards ── */
.scan-cards {
  display: grid;
  grid-template-columns: repeat(3, 200px);
  gap: 20px;
}

.scan-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 28px 20px 24px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 16px;
  cursor: pointer;
  text-align: center;
  transition: border-color 0.15s, background 0.15s, transform 0.1s;
  font-family: inherit;
  color: var(--text);
}
.scan-card:hover {
  border-color: var(--accent);
  background: rgba(88, 166, 255, 0.04);
  transform: translateY(-2px);
}
.scan-card:active { transform: translateY(0); }
.scan-card:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }

.card-icon-wrap {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.card-icon-wrap.accent-blue {
  background: rgba(88, 166, 255, 0.12);
  border: 1px solid rgba(88, 166, 255, 0.22);
  color: var(--accent);
}
.card-icon-wrap.accent-teal {
  background: rgba(63, 185, 80, 0.10);
  border: 1px solid rgba(63, 185, 80, 0.20);
  color: var(--green);
}
.card-icon-wrap.accent-purple {
  background: rgba(180, 120, 255, 0.10);
  border: 1px solid rgba(180, 120, 255, 0.20);
  color: #b478ff;
}
.card-icon { width: 40px; height: 40px; }

.card-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}
.card-desc {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.5;
}

/* ── Results ── */
.rescan-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.rescan-info { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--muted); }
.rescan-stat { font-size: 12px; }
.rescan-stat b { color: var(--text); font-variant-numeric: tabular-nums; }
.rescan-actions { display: flex; gap: 8px; }

.threat-list-wrap { flex: 1; overflow-y: auto; }

.clean-shield svg { width: 64px; height: 64px; }

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
thead {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--surface);
}
th {
  padding: 8px 14px;
  text-align: left;
  font-weight: 500;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}
td {
  padding: 10px 14px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.5);
  vertical-align: middle;
}
.threat-row { transition: background 0.1s; cursor: pointer; }
.threat-row:hover { background: var(--surface-2); }
.threat-row.neutralized { opacity: 0.45; }
.threat-row.selected { background: rgba(88, 166, 255, 0.07); }

.threat-name { min-width: 200px; max-width: 260px; }
.tname {
  display: block;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.treason {
  display: block;
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.type-tag {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 2px 7px;
  border-radius: 4px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--muted);
  white-space: nowrap;
}

.sev-badge {
  display: inline-block;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  padding: 2px 7px;
  border-radius: 4px;
  white-space: nowrap;
}
.sev-badge.critical,
.sev-badge.high {
  background: var(--red-dim);
  color: var(--red);
  border: 1px solid rgba(248, 81, 73, 0.25);
}
.sev-badge.medium {
  background: rgba(210, 153, 34, 0.12);
  color: var(--orange);
  border: 1px solid rgba(210, 153, 34, 0.25);
}
.sev-badge.low {
  background: var(--surface-2);
  color: var(--muted);
  border: 1px solid var(--border);
}

.path-cell {
  color: var(--muted);
  max-width: 300px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.size-cell { color: var(--muted); white-space: nowrap; }

.action-cell { white-space: nowrap; text-align: right; min-width: 130px; }
.action-group { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
.action-error { font-size: 10px; color: var(--red); max-width: 200px; text-align: right; }
.neutralized-label {
  font-size: 11px;
  color: var(--green);
  font-weight: 500;
}

.tag {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--border);
  background: var(--surface-2);
}
.tag.warn { color: var(--orange); border-color: rgba(210, 153, 34, 0.3); background: rgba(210, 153, 34, 0.12); }

/* ── Buttons ── */
.btn {
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  border: 1px solid var(--border);
  transition: filter 0.1s, background 0.1s;
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
.btn.danger {
  background: var(--red);
  border-color: var(--red);
  color: #fff;
  font-weight: 600;
}
.btn.danger:hover { filter: brightness(1.1); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* ── Issue status bar ── */
.issue-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 24px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  font-size: 13px;
}
.issue-bar.warn {
  background: rgba(248, 81, 73, 0.06);
  border-top-color: rgba(248, 81, 73, 0.2);
}
.issue-bar.clean {
  background: rgba(63, 185, 80, 0.05);
  border-top-color: rgba(63, 185, 80, 0.15);
  color: var(--muted);
}
.issue-icon { width: 20px; height: 20px; flex-shrink: 0; }
.issue-text { color: var(--text); }
.issue-text b { font-weight: 600; }

/* ── Modal ── */
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
  border-radius: 12px;
  width: 480px;
  max-width: 90vw;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.4);
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
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.modal-hint { font-size: 12px; color: var(--muted); }
.path-textarea {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 12px;
  padding: 8px 10px;
  resize: vertical;
  outline: none;
  font-family: 'SF Mono', 'Menlo', monospace;
  width: 100%;
  line-height: 1.6;
}
.path-textarea:focus { border-color: var(--accent); }
.path-textarea::placeholder { color: var(--muted); }
.modal-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.mono { font-family: 'SF Mono', 'Menlo', monospace; }

/* ── Pane nav ── */
.pane-nav {
  display: flex;
  gap: 2px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}

.pane-tab {
  padding: 5px 14px;
  border-radius: 6px;
  border: none;
  background: transparent;
  font-family: inherit;
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.pane-tab:hover { background: var(--surface-2); color: var(--text); }
.pane-tab.active { background: var(--surface-2); color: var(--text); }

/* ── Reports pane ── */
.reports-pane {
  flex: 1;
  display: flex;
  flex-direction: row;
  overflow: hidden;
}

.reports-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.report-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.report-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.15s;
}
.report-card:hover { border-color: rgba(88, 166, 255, 0.3); }
.report-card.expanded { border-color: rgba(88, 166, 255, 0.4); }

.report-summary {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  background: transparent;
  border: none;
  font-family: inherit;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  text-align: left;
}
.report-summary:hover { background: var(--surface-2); }

.report-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.report-dot.status-clean { background: var(--green); }
.report-dot.status-ok { background: var(--orange); }
.report-dot.status-danger { background: var(--red); }

.report-date {
  font-weight: 500;
  color: var(--text);
  min-width: 140px;
}

.report-type-badge {
  text-transform: capitalize;
  font-size: 10px;
  font-weight: 500;
  background: rgba(88, 166, 255, 0.10);
  color: var(--accent);
  border: 1px solid rgba(88, 166, 255, 0.2);
  padding: 1px 7px;
  border-radius: 8px;
}

.report-stat { color: var(--muted); }
.report-stat b { color: var(--text); font-variant-numeric: tabular-nums; }

.neutralized-stat { color: var(--green); }

.report-elapsed {
  margin-left: auto;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.report-chevron {
  font-size: 16px;
  color: var(--muted);
  transition: transform 0.15s;
  line-height: 1;
  margin-left: 4px;
}
.report-chevron.open { transform: rotate(90deg); }

.report-detail {
  border-top: 1px solid var(--border);
  background: var(--surface-2);
}

.report-clean {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  font-size: 12px;
  color: var(--green);
}

.report-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.report-table th {
  padding: 7px 14px;
  text-align: left;
  font-size: 10px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
  background: transparent;
  position: static;
}
.report-table td {
  padding: 8px 14px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.4);
  vertical-align: middle;
}
.report-table tr:last-child td { border-bottom: none; }

.active-threat-label {
  font-size: 11px;
  color: var(--red);
  font-weight: 500;
}

/* ── Pane badge ── */
.pane-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  font-size: 10px;
  font-weight: 700;
  background: var(--red);
  color: #fff;
  margin-left: 5px;
  line-height: 1;
}

/* ── Intruder pane ── */
.intruder-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Hero / idle state */
.intruder-hero {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 18px;
  padding: 48px 40px 56px;
  background: radial-gradient(ellipse 80% 60% at 50% 0%, rgba(88,166,255,0.08) 0%, transparent 70%);
}

.radar-idle svg,
.radar-idle {
  width: 120px;
  height: 120px;
}

.intruder-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.3px;
}

.intruder-desc {
  font-size: 13px;
  color: var(--muted);
  max-width: 440px;
  text-align: center;
  line-height: 1.6;
}

.intruder-techniques {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  max-width: 480px;
}

.technique-tag {
  font-size: 11px;
  padding: 3px 9px;
  border-radius: 10px;
  background: rgba(88,166,255,0.08);
  border: 1px solid rgba(88,166,255,0.18);
  color: var(--accent);
}

.intruder-run-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
  padding: 10px 24px;
  font-size: 13px;
  font-weight: 600;
  background: var(--accent);
  border-color: var(--accent);
  color: #0d1117;
  border-radius: 8px;
}
.intruder-run-btn:hover { filter: brightness(1.08); }

/* Radar animation (running state) */
.radar-running {
  width: 120px;
  height: 120px;
  position: relative;
}
.radar-running svg {
  width: 120px;
  height: 120px;
}

@keyframes radar-spin {
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
}

.radar-sweep-arm {
  transform-origin: 60px 60px;
  animation: radar-spin 2s linear infinite;
}

/* Results bar */
.intruder-results-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.irb-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.irb-count {
  font-size: 12px;
  color: var(--muted);
}
.irb-count b { color: var(--text); }

.irb-stat {
  font-size: 12px;
  color: var(--muted);
}

.sev-pill {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 2px 7px;
  border-radius: 8px;
}
.sev-pill.critical {
  background: var(--red-dim);
  color: var(--red);
}
.sev-pill.high {
  background: rgba(210,153,34,0.12);
  color: var(--orange);
}
.sev-pill.medium {
  background: var(--surface-2);
  color: var(--muted);
}

/* Findings two-column layout */
.intruder-findings-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Finding list */
.finding-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}

.finding-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: 10px;
  padding: 14px 16px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.12s;
}
.finding-card:hover  { background: rgba(88,166,255,0.03); }
.finding-card.selected { background: rgba(88,166,255,0.07); border-color: rgba(88,166,255,0.4); }
.finding-card.critical { border-left-color: var(--red); }
.finding-card.high     { border-left-color: var(--orange); }
.finding-card.medium   { border-left-color: #d29922; }
.finding-card.low      { border-left-color: var(--border); }

.fc-top {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.fc-proc {
  font-size: 11px;
  color: var(--text);
  font-weight: 500;
  margin-left: auto;
}
.fc-pid {
  color: var(--muted);
  font-weight: 400;
  margin-left: 4px;
}

.fc-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 4px;
  line-height: 1.4;
}

.fc-desc {
  font-size: 12px;
  color: var(--muted);
  line-height: 1.55;
}

/* Panel slide transition */
.panel-enter-active, .panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-enter-from, .panel-leave-to { transform: translateX(100%); opacity: 0; }

/* ── Full Disk Access banner ── */
.fda-banner {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 14px 20px;
  background: rgba(210, 153, 34, 0.08);
  border: 1px solid rgba(210, 153, 34, 0.28);
  border-radius: 12px;
  width: 100%;
  max-width: 680px;
  text-align: left;
}

.fda-icon {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  margin-top: 1px;
}
.fda-icon svg { width: 20px; height: 20px; }

.fda-body { flex: 1; min-width: 0; }

.fda-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--orange);
  margin-bottom: 4px;
}

.fda-desc {
  font-size: 12px;
  color: var(--muted);
  line-height: 1.55;
}

.fda-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.fda-btn {
  white-space: nowrap;
  font-size: 12px;
  padding: 6px 12px;
}

.fda-dismiss {
  padding: 5px 8px;
  font-size: 11px;
  color: var(--muted);
  border-color: transparent;
  background: transparent;
}
.fda-dismiss:hover { background: var(--surface-2); border-color: var(--border); }

.fda-banner-enter-active,
.fda-banner-leave-active { transition: opacity 0.2s, transform 0.2s; }
.fda-banner-enter-from,
.fda-banner-leave-to { opacity: 0; transform: translateY(-6px); }

/* Category tag (distinct from type-tag) */
.cat-tag {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 2px 7px;
  border-radius: 4px;
  background: rgba(180,120,255,0.10);
  border: 1px solid rgba(180,120,255,0.22);
  color: #b478ff;
  white-space: nowrap;
}
</style>
