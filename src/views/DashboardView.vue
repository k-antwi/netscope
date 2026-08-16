<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SystemMetrics } from '../types'
import { useOutbound } from '../composables/useOutbound'
import { useInbound } from '../composables/useInbound'
import { useAlerts } from '../composables/useAlerts'
import { useBrowser } from '../composables/useBrowser'

const HISTORY = 24

// --- System metrics ---
const sysMetrics = ref<SystemMetrics | null>(null)
const cpuHistory = ref<number[]>(Array(HISTORY).fill(0))
const netInHistory = ref<number[]>(Array(HISTORY).fill(0))
const netOutHistory = ref<number[]>(Array(HISTORY).fill(0))
let prevNetIn = 0
let prevNetOut = 0
let prevNetTime = 0
let sysTimer: ReturnType<typeof setInterval> | null = null

// --- App data composables ---
const { connections: outbound } = useOutbound()
const { connections: inbound } = useInbound()
const { issues } = useAlerts()
const { requests: browserRequests, totalBytes: browserBytes, extensionConnected } = useBrowser()

async function fetchMetrics() {
  try {
    const m = await invoke<SystemMetrics>('get_system_metrics')
    sysMetrics.value = m
    cpuHistory.value = [...cpuHistory.value, m.cpu_percent].slice(-HISTORY)
    const now = Date.now()
    if (prevNetTime > 0) {
      const dt = (now - prevNetTime) / 1000
      const inKb = Math.max(0, (m.net_in_bytes - prevNetIn) / 1024 / dt)
      const outKb = Math.max(0, (m.net_out_bytes - prevNetOut) / 1024 / dt)
      netInHistory.value = [...netInHistory.value, inKb].slice(-HISTORY)
      netOutHistory.value = [...netOutHistory.value, outKb].slice(-HISTORY)
    }
    prevNetIn = m.net_in_bytes
    prevNetOut = m.net_out_bytes
    prevNetTime = now
  } catch { /* ignore errors silently */ }
}

onMounted(() => { fetchMetrics(); sysTimer = setInterval(fetchMetrics, 3000) })
onUnmounted(() => { if (sysTimer) clearInterval(sysTimer) })

// --- System metric accessors ---
const cpuPct    = computed(() => sysMetrics.value?.cpu_percent ?? 0)
const memUsed   = computed(() => sysMetrics.value?.memory_used_gb ?? 0)
const memTotal  = computed(() => sysMetrics.value?.memory_total_gb ?? 1)
const memPct    = computed(() => Math.min((memUsed.value / memTotal.value) * 100, 100))
const netIn     = computed(() => netInHistory.value[netInHistory.value.length - 1] ?? 0)
const netOut    = computed(() => netOutHistory.value[netOutHistory.value.length - 1] ?? 0)

// --- App stats ---
const outStats = computed(() => {
  const c = outbound.value
  return {
    total: c.length,
    httpsPct: c.length ? Math.round(c.filter(x => x.is_https).length / c.length * 100) : 0,
    processes: new Set(c.map(x => x.process)).size,
  }
})

const inStats = computed(() => {
  const listening = inbound.value.filter(x => x.state === 'LISTEN')
  return {
    listening: listening.length,
    exposed: listening.filter(x => x.is_all_interfaces).length,
    encrypted: listening.filter(x => x.is_encrypted).length,
  }
})

const alertStats = computed(() => ({
  critical: issues.value.filter(i => i.severity === 'critical').length,
  high:     issues.value.filter(i => i.severity === 'high').length,
  warning:  issues.value.filter(i => i.severity === 'warning').length,
  info:     issues.value.filter(i => i.severity === 'info').length,
  total:    issues.value.length,
}))

const browserStats = computed(() => ({
  total:     browserRequests.value.length,
  errors:    browserRequests.value.filter(r => r.error || r.status >= 400).length,
  mb:        (browserBytes.value / (1024 * 1024)).toFixed(1),
  connected: extensionConnected.value,
}))

const topProcesses = computed(() => {
  const counts = new Map<string, number>()
  for (const c of outbound.value) counts.set(c.process, (counts.get(c.process) || 0) + 1)
  const sorted = [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6)
  const max = sorted[0]?.[1] || 1
  return sorted.map(([name, count]) => ({ name, count, pct: (count / max) * 100 }))
})

const recentAlerts = computed(() => issues.value.slice(0, 6))

// --- SVG helpers ---
function sparkPath(data: number[], w: number, h: number, fill: boolean): string {
  if (data.length < 2) return ''
  const max = Math.max(...data, 1)
  const pts = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w
    const y = h - (v / max) * (h - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  if (!fill) return `M ${pts.join(' L ')}`
  return `M ${pts[0]} L ${pts.join(' L ')} L ${w},${h} L 0,${h} Z`
}

const GAUGE_R    = 36
const GAUGE_CIRC = 2 * Math.PI * GAUGE_R

function gaugeDash(pct: number) {
  const filled = (Math.min(pct, 100) / 100) * GAUGE_CIRC
  return `${filled.toFixed(1)} ${(GAUGE_CIRC + 1).toFixed(1)}`
}

function gaugeStroke(pct: number) {
  if (pct < 60) return '#3fb950'
  if (pct < 80) return '#d29922'
  return '#f85149'
}

function fmtNet(kb: number) {
  if (kb < 1024) return `${kb.toFixed(1)} KB/s`
  return `${(kb / 1024).toFixed(2)} MB/s`
}

function sevColor(sev: string) {
  return sev === 'critical' ? '#f85149' : sev === 'high' ? '#d29922' : sev === 'warning' ? '#e3b341' : '#58a6ff'
}
</script>

<template>
  <div class="dashboard">
    <div class="dash-header">
      <div>
        <h1 class="dash-title">Dashboard</h1>
        <p class="dash-sub">Live system &amp; network overview</p>
      </div>
    </div>

    <!-- ── Row 1: System Vitals ── -->
    <div class="vitals-row">

      <!-- CPU -->
      <div class="card vitals-card">
        <div class="card-label">CPU Usage</div>
        <div class="cpu-body">
          <div class="gauge-wrap">
            <svg viewBox="0 0 90 90" width="90" height="90">
              <circle cx="45" cy="45" :r="GAUGE_R" fill="none" stroke="var(--surface-2)" stroke-width="7"/>
              <circle cx="45" cy="45" :r="GAUGE_R" fill="none"
                :stroke="gaugeStroke(cpuPct)" stroke-width="7"
                stroke-linecap="round"
                :stroke-dasharray="gaugeDash(cpuPct)"
                transform="rotate(-90 45 45)"
              />
              <text x="45" y="50" text-anchor="middle" class="gauge-num">{{ cpuPct.toFixed(0) }}%</text>
            </svg>
          </div>
          <div class="spark-col">
            <svg class="sparkline" viewBox="0 0 180 44" preserveAspectRatio="none">
              <defs>
                <linearGradient id="cpuFill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#58a6ff" stop-opacity="0.35"/>
                  <stop offset="100%" stop-color="#58a6ff" stop-opacity="0"/>
                </linearGradient>
              </defs>
              <path :d="sparkPath(cpuHistory, 180, 44, true)" fill="url(#cpuFill)"/>
              <path :d="sparkPath(cpuHistory, 180, 44, false)" fill="none" stroke="#58a6ff" stroke-width="1.5" stroke-linejoin="round"/>
            </svg>
            <span class="spark-label">last {{ HISTORY * 3 }}s</span>
          </div>
        </div>
      </div>

      <!-- Memory -->
      <div class="card vitals-card">
        <div class="card-label">Memory</div>
        <div class="mem-body">
          <svg viewBox="0 0 90 90" width="110" height="110">
            <circle cx="45" cy="45" :r="GAUGE_R" fill="none" stroke="var(--surface-2)" stroke-width="7"/>
            <circle cx="45" cy="45" :r="GAUGE_R" fill="none"
              :stroke="gaugeStroke(memPct)" stroke-width="7"
              stroke-linecap="round"
              :stroke-dasharray="gaugeDash(memPct)"
              transform="rotate(-90 45 45)"
            />
            <text x="45" y="42" text-anchor="middle" class="gauge-num-lg">{{ memUsed.toFixed(1) }}</text>
            <text x="45" y="56" text-anchor="middle" class="gauge-unit">GB</text>
          </svg>
          <div class="mem-info">
            <span class="mem-of">of {{ memTotal.toFixed(0) }} GB</span>
            <span class="mem-pct">{{ memPct.toFixed(0) }}% used</span>
          </div>
        </div>
      </div>

      <!-- Network I/O -->
      <div class="card vitals-card">
        <div class="card-label">Network I/O</div>
        <div class="net-body">
          <div class="net-row">
            <svg class="sparkline-sm" viewBox="0 0 140 34" preserveAspectRatio="none">
              <defs>
                <linearGradient id="netUpFill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#d29922" stop-opacity="0.35"/>
                  <stop offset="100%" stop-color="#d29922" stop-opacity="0"/>
                </linearGradient>
              </defs>
              <path :d="sparkPath(netOutHistory, 140, 34, true)" fill="url(#netUpFill)"/>
              <path :d="sparkPath(netOutHistory, 140, 34, false)" fill="none" stroke="#d29922" stroke-width="1.5" stroke-linejoin="round"/>
            </svg>
            <div class="net-stat">
              <span class="net-val">{{ fmtNet(netOut) }}</span>
              <span class="net-dir" style="color:#d29922">↑ Upload</span>
            </div>
          </div>
          <div class="net-divider"/>
          <div class="net-row">
            <svg class="sparkline-sm" viewBox="0 0 140 34" preserveAspectRatio="none">
              <defs>
                <linearGradient id="netDnFill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#3fb950" stop-opacity="0.35"/>
                  <stop offset="100%" stop-color="#3fb950" stop-opacity="0"/>
                </linearGradient>
              </defs>
              <path :d="sparkPath(netInHistory, 140, 34, true)" fill="url(#netDnFill)"/>
              <path :d="sparkPath(netInHistory, 140, 34, false)" fill="none" stroke="#3fb950" stroke-width="1.5" stroke-linejoin="round"/>
            </svg>
            <div class="net-stat">
              <span class="net-val">{{ fmtNet(netIn) }}</span>
              <span class="net-dir" style="color:#3fb950">↓ Download</span>
            </div>
          </div>
        </div>
      </div>

    </div>

    <!-- ── Row 2: App Activity ── -->
    <div class="activity-row">

      <!-- Outbound -->
      <div class="card activity-card">
        <div class="act-icon act-out">
          <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 13L13 5M13 5H7.5M13 5v5.5"/>
          </svg>
        </div>
        <div class="act-num">{{ outStats.total }}</div>
        <div class="act-label">Outbound</div>
        <div class="act-pills">
          <span class="pill green">{{ outStats.httpsPct }}% HTTPS</span>
          <span class="pill blue">{{ outStats.processes }} apps</span>
        </div>
      </div>

      <!-- Inbound -->
      <div class="card activity-card">
        <div class="act-icon act-in">
          <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M13 5L5 13M5 13h5.5M5 13V7.5"/>
          </svg>
        </div>
        <div class="act-num" :class="{ 'text-red': inStats.exposed > 0 }">{{ inStats.listening }}</div>
        <div class="act-label">Inbound</div>
        <div class="act-pills">
          <span v-if="inStats.exposed" class="pill red">{{ inStats.exposed }} exposed</span>
          <span class="pill green">{{ inStats.encrypted }} encrypted</span>
        </div>
      </div>

      <!-- Alerts -->
      <div class="card activity-card">
        <div class="act-icon act-alert">
          <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 2a4.5 4.5 0 0 0-4.5 4.5c0 5-2 6-2 6h13s-2-1-2-6A4.5 4.5 0 0 0 9 2z"/>
            <path d="M10.2 15.5a1.3 1.3 0 0 1-2.4 0"/>
          </svg>
        </div>
        <div class="act-num" :class="alertStats.critical > 0 ? 'text-red' : alertStats.high > 0 ? 'text-orange' : ''">
          {{ alertStats.total }}
        </div>
        <div class="act-label">Alerts</div>
        <div class="sev-row">
          <span class="sev-pip" style="color:#f85149">● {{ alertStats.critical }}</span>
          <span class="sev-pip" style="color:#d29922">● {{ alertStats.high }}</span>
          <span class="sev-pip" style="color:#e3b341">● {{ alertStats.warning }}</span>
          <span class="sev-pip" style="color:#58a6ff">● {{ alertStats.info }}</span>
        </div>
      </div>

      <!-- Browser -->
      <div class="card activity-card">
        <div class="act-icon act-browser">
          <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="9" cy="9" r="7"/>
            <line x1="2" y1="9" x2="16" y2="9"/>
            <path d="M9 2a11 11 0 0 1 2.5 7A11 11 0 0 1 9 16a11 11 0 0 1-2.5-7A11 11 0 0 1 9 2z"/>
          </svg>
        </div>
        <div class="act-num">{{ browserStats.total }}</div>
        <div class="act-label">Browser Reqs</div>
        <div class="act-pills">
          <span class="pill" :class="browserStats.connected ? 'green' : 'muted'">
            {{ browserStats.connected ? '● Live' : '○ Offline' }}
          </span>
          <span class="pill blue">{{ browserStats.mb }} MB</span>
        </div>
      </div>

    </div>

    <!-- ── Row 3: Details ── -->
    <div class="detail-row">

      <!-- Top Processes -->
      <div class="card detail-card">
        <div class="detail-head">
          <span class="detail-title">Top Processes</span>
          <span class="detail-sub">by outbound connections</span>
        </div>
        <div class="proc-list" v-if="topProcesses.length">
          <div class="proc-row" v-for="p in topProcesses" :key="p.name">
            <span class="proc-name">{{ p.name }}</span>
            <div class="proc-track">
              <div class="proc-bar" :style="{ width: p.pct + '%' }"/>
            </div>
            <span class="proc-cnt">{{ p.count }}</span>
          </div>
        </div>
        <div class="empty" v-else>No active connections</div>
      </div>

      <!-- Recent Alerts -->
      <div class="card detail-card">
        <div class="detail-head">
          <span class="detail-title">Recent Alerts</span>
          <span class="detail-sub">latest issues detected</span>
        </div>
        <div class="alert-list" v-if="recentAlerts.length">
          <div class="alert-row" v-for="(a, i) in recentAlerts" :key="i">
            <span class="alert-dot" :style="{ color: sevColor(a.severity) }">●</span>
            <div class="alert-body">
              <span class="alert-ttl">{{ a.title }}</span>
              <span class="alert-proc">{{ a.process }}</span>
            </div>
            <span class="alert-sev" :style="{ color: sevColor(a.severity) }">{{ a.severity }}</span>
          </div>
        </div>
        <div class="empty" v-else>No alerts detected</div>
      </div>

    </div>
  </div>
</template>

<style scoped>
/* ── Layout ── */
.dashboard {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px 24px 24px;
  overflow-y: auto;
  background: var(--bg);
  min-width: 0;
}

.dash-header {
  padding-top: 6px;
  flex-shrink: 0;
}

.dash-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: 0.01em;
}

.dash-sub {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
}

/* ── Shared card ── */
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
}

/* ── Vitals row ── */
.vitals-row {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 12px;
  flex-shrink: 0;
}

.vitals-card {
  padding: 16px;
}

.card-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-bottom: 14px;
}

/* CPU card */
.cpu-body {
  display: flex;
  align-items: center;
  gap: 12px;
}

.gauge-wrap { flex-shrink: 0; }

.gauge-num {
  font-size: 13px;
  font-weight: 700;
  fill: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.spark-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.sparkline {
  width: 100%;
  height: 44px;
  display: block;
  border-radius: 4px;
  overflow: hidden;
}

.spark-label {
  font-size: 10px;
  color: var(--muted);
  opacity: 0.6;
}

/* Memory card */
.mem-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.gauge-num-lg {
  font-size: 15px;
  font-weight: 700;
  fill: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.gauge-unit {
  font-size: 10px;
  fill: var(--muted);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.mem-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.mem-of {
  font-size: 11px;
  color: var(--muted);
}

.mem-pct {
  font-size: 11px;
  color: var(--text);
  font-weight: 600;
}

/* Network card */
.net-body {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.net-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 0;
}

.sparkline-sm {
  flex: 1;
  height: 34px;
  display: block;
  min-width: 0;
}

.net-stat {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex-shrink: 0;
  width: 80px;
}

.net-val {
  font-size: 12px;
  font-weight: 700;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.net-dir {
  font-size: 10px;
  font-weight: 500;
}

.net-divider {
  height: 1px;
  background: var(--border);
  margin: 4px 0;
}

/* ── Activity row ── */
.activity-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  flex-shrink: 0;
}

.activity-card {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.act-icon {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
  flex-shrink: 0;
}

.act-icon svg { width: 16px; height: 16px; }

.act-out     { background: rgba(88, 166, 255, 0.12); color: #58a6ff; }
.act-in      { background: rgba(63, 185, 80, 0.12);  color: #3fb950; }
.act-alert   { background: rgba(248, 81, 73, 0.12);  color: #f85149; }
.act-browser { background: rgba(210, 153, 34, 0.12); color: #d29922; }

.act-num {
  font-size: 28px;
  font-weight: 700;
  color: var(--text);
  line-height: 1;
  font-variant-numeric: tabular-nums;
}

.act-label {
  font-size: 11px;
  color: var(--muted);
  margin-bottom: 8px;
}

.act-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.pill {
  font-size: 10px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 10px;
  border: 1px solid transparent;
}

.pill.green  { color: #3fb950; background: rgba(63, 185, 80, 0.1);  border-color: rgba(63, 185, 80, 0.25); }
.pill.blue   { color: #58a6ff; background: rgba(88, 166, 255, 0.1); border-color: rgba(88, 166, 255, 0.25); }
.pill.red    { color: #f85149; background: rgba(248, 81, 73, 0.1);  border-color: rgba(248, 81, 73, 0.25); }
.pill.muted  { color: var(--muted); background: transparent; border-color: var(--border); }

.sev-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.sev-pip {
  font-size: 11px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.text-red    { color: #f85149; }
.text-orange { color: #d29922; }

/* ── Detail row ── */
.detail-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.detail-card {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
}

.detail-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.detail-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.detail-sub {
  font-size: 10px;
  color: var(--muted);
}

/* Process bars */
.proc-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.proc-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.proc-name {
  font-size: 11px;
  color: var(--text);
  width: 90px;
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.proc-track {
  flex: 1;
  height: 5px;
  background: var(--surface-2);
  border-radius: 3px;
  overflow: hidden;
}

.proc-bar {
  height: 100%;
  background: linear-gradient(90deg, #58a6ff, #3fb950);
  border-radius: 3px;
  transition: width 0.4s ease;
}

.proc-cnt {
  font-size: 11px;
  color: var(--muted);
  width: 24px;
  text-align: right;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

/* Alert list */
.alert-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
}

.alert-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  background: var(--surface-2);
}

.alert-dot {
  font-size: 10px;
  flex-shrink: 0;
}

.alert-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.alert-ttl {
  font-size: 11px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.alert-proc {
  font-size: 10px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.alert-sev {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  flex-shrink: 0;
}

.empty {
  font-size: 12px;
  color: var(--muted);
  text-align: center;
  padding: 20px 0;
  opacity: 0.6;
}
</style>
