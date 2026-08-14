<script setup lang="ts">
import type { ServiceInvestigation } from '../types'
import { PORT_LABELS } from '../types'

defineProps<{
  localIp: string
  localPort: number
  investigation: ServiceInvestigation | null
  isLoading: boolean
}>()

defineEmits<{ close: [] }>()

function portLabel(port: number): string {
  return PORT_LABELS[port] ? `${port} · ${PORT_LABELS[port]}` : String(port)
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
</style>
