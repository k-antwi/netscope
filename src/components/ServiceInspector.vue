<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ServiceInvestigation, CveCheckResult } from '../types'
import { PORT_LABELS } from '../types'

const props = defineProps<{
  localIp: string
  localPort: number
  investigation: ServiceInvestigation | null
  isLoading: boolean
}>()

defineEmits<{ close: [] }>()

function portLabel(port: number): string {
  return PORT_LABELS[port] ? `${port} · ${PORT_LABELS[port]}` : String(port)
}

function guessCveQuery(inv: ServiceInvestigation | null): string {
  if (!inv) return ''
  const firstToken = (inv.process_path || '').trim().split(/\s+/)[0] || ''
  const exeName = firstToken.split('/').pop() || ''
  return [exeName, inv.service_name].filter(Boolean).join(' ').trim()
}

const cveQuery = ref('')
const isCheckingCve = ref(false)
const cveResult = ref<CveCheckResult | null>(null)
const cveError = ref<string | null>(null)

watch(
  () => props.investigation,
  (inv) => {
    cveQuery.value = guessCveQuery(inv)
    cveResult.value = null
    cveError.value = null
    isCheckingCve.value = false
  },
  { immediate: true },
)

async function checkCves() {
  const query = cveQuery.value.trim()
  if (!query) return
  isCheckingCve.value = true
  cveError.value = null
  cveResult.value = null
  try {
    cveResult.value = await invoke<CveCheckResult>('check_cves', { query })
  } catch (e) {
    cveError.value = String(e)
  } finally {
    isCheckingCve.value = false
  }
}

function severityColor(severity: string | null): string {
  if (!severity) return 'muted'
  const s = severity.toUpperCase()
  if (s === 'CRITICAL' || s === 'HIGH') return 'orange'
  if (s === 'MEDIUM') return 'orange'
  return 'muted'
}
</script>

<template>
  <aside class="inspector">
    <div class="inspector-header">
      <div>
        <div class="inspector-title">Service Investigation</div>
        <div class="inspector-ip mono">
          {{ localIp === '*' ? 'all interfaces' : localIp }}:{{ localPort }}
        </div>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <div v-if="isLoading" class="loading">
      <span class="spin-lg">↺</span>
      <span>Inspecting service…</span>
      <span class="hint">tracing active connections</span>
    </div>

    <div v-else-if="investigation" class="results">

      <!-- Verdict -->
      <div v-if="investigation.warnings.length" class="verdict suspicious">
        <span class="verdict-icon">⚠</span>
        <span>{{ investigation.warnings.length }} warning{{ investigation.warnings.length > 1 ? 's' : '' }}</span>
      </div>
      <div v-else class="verdict clean">
        <span class="verdict-icon">✓</span>
        <span>No warnings</span>
      </div>

      <ul v-if="investigation.warnings.length" class="reasons">
        <li v-for="w in investigation.warnings" :key="w">{{ w }}</li>
      </ul>

      <div class="divider" />

      <!-- Service identity -->
      <div class="section-title">Service</div>
      <div class="field-grid">
        <div class="field">
          <span class="label">Port</span>
          <span class="value mono">{{ portLabel(investigation.local_port) }}</span>
        </div>
        <div class="field">
          <span class="label">Process</span>
          <span class="value mono small">{{ investigation.process_path || '—' }}</span>
        </div>
        <div class="field">
          <span class="label">PID</span>
          <span class="value mono">{{ investigation.pid }}</span>
        </div>
      </div>

      <div class="divider" />

      <!-- CVE check -->
      <div class="section-title">CVE Check</div>
      <div class="cve-search-row">
        <input
          v-model="cveQuery"
          class="cve-input mono"
          placeholder="e.g. OpenSSH 8.2"
          @keyup.enter="checkCves"
        />
        <button
          class="cve-btn"
          :disabled="isCheckingCve || !cveQuery.trim()"
          @click="checkCves"
        >{{ isCheckingCve ? 'Searching…' : 'Search CVEs' }}</button>
      </div>
      <div class="cve-hint">
        Free-text match against the NVD — not version-specific, review results before acting on them.
      </div>

      <div v-if="cveError" class="cve-result">
        <span class="value warn">Search failed</span>
        <span class="value small">{{ cveError }}</span>
      </div>

      <template v-else-if="cveResult">
        <div v-if="cveResult.message" class="cve-result">
          <span class="value small">{{ cveResult.message }}</span>
        </div>
        <div v-else-if="cveResult.cves.length === 0" class="cve-result">
          <span class="value small">No matching CVEs found for "{{ cveResult.query }}".</span>
        </div>
        <div v-else class="cve-list">
          <a
            v-for="c in cveResult.cves"
            :key="c.id"
            :href="c.url"
            target="_blank"
            rel="noopener noreferrer"
            class="cve-card"
          >
            <div class="cve-card-head">
              <span class="cve-id mono">{{ c.id }}</span>
              <span v-if="c.severity" class="value" :class="severityColor(c.severity)">
                {{ c.severity }}{{ c.score ? ` · ${c.score}` : '' }}
              </span>
            </div>
            <div class="cve-desc">{{ c.description }}</div>
          </a>
          <div v-if="cveResult.total_results > cveResult.cves.length" class="cve-more">
            Showing {{ cveResult.cves.length }} of {{ cveResult.total_results }} matches — narrow the search above for more precise results.
          </div>
        </div>
      </template>

      <div class="divider" />

      <!-- Exposure -->
      <div class="section-title">Exposure</div>
      <div class="field-grid">
        <div class="field">
          <span class="label">Listening on</span>
          <span
            class="value mono"
            :class="{
              warn: localIp === '*' || localIp === '0.0.0.0',
              muted: localIp === '127.0.0.1' || localIp === '::1',
            }"
          >
            {{ localIp === '*' ? '* (all interfaces)' : localIp }}
          </span>
        </div>
        <div class="field">
          <span class="label">Scope</span>
          <span class="value">{{ investigation.exposure }}</span>
        </div>
        <div class="field">
          <span class="label">Encryption</span>
          <span class="value" :class="investigation.is_encrypted ? 'green' : 'warn'">
            {{ investigation.is_encrypted ? 'Yes (TLS port)' : 'No' }}
          </span>
        </div>
      </div>

      <!-- Active connections -->
      <template v-if="investigation.active_connections > 0">
        <div class="divider" />

        <div class="section-title">
          Active Connections
          <span class="count-badge">{{ investigation.active_connections }}</span>
        </div>

        <div
          v-for="trace in investigation.active_remotes"
          :key="trace.ip"
          class="trace-card"
        >
          <div class="trace-ip mono">{{ trace.ip }}</div>
          <div class="trace-row">
            <span class="label">Hostname</span>
            <span class="value mono small">{{ trace.rdns || '—' }}</span>
          </div>
          <div class="trace-row">
            <span class="label">Organisation</span>
            <span class="value small">{{ trace.org || '—' }}</span>
          </div>
          <div class="trace-row" v-if="trace.city || trace.country">
            <span class="label">Location</span>
            <span class="value small">
              {{ [trace.city, trace.country].filter(Boolean).join(', ') }}
            </span>
          </div>
        </div>
      </template>

      <div v-else class="no-connections">
        No active connections on this port
      </div>

    </div>
  </aside>
</template>

<style scoped>
.inspector {
  width: var(--panel-w);
  flex-shrink: 0;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.inspector-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 16px;
  border-bottom: 1px solid var(--border);
  gap: 8px;
}

.inspector-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
  margin-bottom: 4px;
}

.inspector-ip {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

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

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 10px;
  color: var(--muted);
  font-size: 13px;
}

@keyframes spin { to { transform: rotate(360deg); } }
.spin-lg { display: inline-block; animation: spin 1s linear infinite; font-size: 22px; }
.hint { font-size: 11px; font-family: 'SF Mono', 'Menlo', monospace; }

.results { padding: 12px; display: flex; flex-direction: column; gap: 12px; }

.verdict {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px;
  font-weight: 600;
  font-size: 13px;
}
.verdict.clean {
  background: var(--green-dim);
  color: var(--green);
  border: 1px solid rgba(63, 185, 80, 0.25);
}
.verdict.suspicious {
  background: rgba(210, 153, 34, 0.1);
  color: var(--orange);
  border: 1px solid rgba(210, 153, 34, 0.3);
}
.verdict-icon { font-size: 16px; }

.reasons {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.reasons li {
  font-size: 12px;
  color: var(--orange);
  padding-left: 14px;
  position: relative;
}
.reasons li::before { content: '•'; position: absolute; left: 4px; }

.divider { height: 1px; background: var(--border); }

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
  margin-bottom: -4px;
}

.count-badge {
  background: var(--accent);
  color: #000;
  font-size: 10px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 10px;
  letter-spacing: 0;
  text-transform: none;
}

.field-grid { display: flex; flex-direction: column; gap: 8px; }
.field { display: flex; flex-direction: column; gap: 2px; }

.label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.value { font-size: 12px; color: var(--text); word-break: break-all; }
.value.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.value.small { font-size: 11px; }
.value.warn { color: var(--orange); }
.value.green { color: var(--green); }
.value.muted { color: var(--muted); }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }

/* Trace cards */
.trace-card {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.trace-ip {
  font-size: 13px;
  font-weight: 600;
  color: var(--accent);
  margin-bottom: 2px;
}

.trace-row {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.no-connections {
  font-size: 12px;
  color: var(--muted);
  text-align: center;
  padding: 8px 0;
}

/* CVE check */
.cve-search-row {
  display: flex;
  gap: 6px;
}

.cve-input {
  flex: 1;
  min-width: 0;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 6px 8px;
  font-size: 12px;
  color: var(--text);
}
.cve-input:focus { outline: none; border-color: var(--accent); }

.cve-btn {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 6px 10px;
  cursor: pointer;
}
.cve-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.cve-btn:disabled { cursor: default; opacity: 0.6; }

.cve-hint {
  font-size: 10px;
  color: var(--muted);
  margin-top: -4px;
  line-height: 1.4;
}

.cve-result {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cve-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.cve-card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 7px;
  text-decoration: none;
}
.cve-card:hover { border-color: var(--accent); }

.cve-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.cve-id {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
}

.cve-desc {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cve-more {
  font-size: 10px;
  color: var(--muted);
  line-height: 1.4;
}
</style>
