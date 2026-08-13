<script setup lang="ts">
import type { IpInvestigation } from '../types'

defineProps<{
  ip: string
  port: number
  investigation: IpInvestigation | null
  isLoading: boolean
}>()

defineEmits<{ close: [] }>()
</script>

<template>
  <aside class="inspector">
    <div class="inspector-header">
      <div>
        <div class="inspector-title">IP Investigation</div>
        <div class="inspector-ip mono">{{ ip }}</div>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <div v-if="isLoading" class="loading">
      <span class="spin-lg">↺</span>
      <span>Running lookups…</span>
      <span class="hint">whois · dig · ipinfo · tls</span>
    </div>

    <div v-else-if="investigation" class="results">
      <!-- Verdict banner -->
      <div class="verdict" :class="investigation.suspicious ? 'suspicious' : 'clean'">
        <span class="verdict-icon">{{ investigation.suspicious ? '⚠' : '✓' }}</span>
        <span>{{ investigation.suspicious ? 'Suspicious' : 'Looks clean' }}</span>
      </div>

      <!-- Suspicious reasons -->
      <ul v-if="investigation.suspicious_reasons.length" class="reasons">
        <li v-for="r in investigation.suspicious_reasons" :key="r">{{ r }}</li>
      </ul>

      <div class="divider" />

      <!-- Network info -->
      <div class="section-title">Network</div>
      <div class="field-grid">
        <div class="field">
          <span class="label">ASN / Org</span>
          <span class="value" :class="{ suspicious: investigation.suspicious }">
            {{ investigation.asn_org || '—' }}
          </span>
        </div>
        <div class="field">
          <span class="label">WHOIS Org</span>
          <span class="value">{{ investigation.whois_org || '—' }}</span>
        </div>
        <div class="field">
          <span class="label">Net Name</span>
          <span class="value mono">{{ investigation.whois_netname || '—' }}</span>
        </div>
        <div class="field">
          <span class="label">Reverse DNS</span>
          <span class="value mono" :class="{ warn: !investigation.rdns }">
            {{ investigation.rdns || 'none' }}
          </span>
        </div>
        <div class="field">
          <span class="label">Location</span>
          <span class="value">
            {{ [investigation.city, investigation.country].filter(Boolean).join(', ') || '—' }}
          </span>
        </div>
      </div>

      <div class="divider" />

      <!-- TLS -->
      <div class="section-title">TLS Certificate</div>
      <div class="field-grid">
        <div class="field">
          <span class="label">Subject</span>
          <span class="value mono small" :class="{ suspicious: investigation.suspicious && investigation.tls_subject }">
            {{ investigation.tls_subject || '—' }}
          </span>
        </div>
        <div class="field">
          <span class="label">Issuer</span>
          <span class="value mono small">{{ investigation.tls_issuer || '—' }}</span>
        </div>
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
  font-size: 15px;
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
  line-height: 1;
  flex-shrink: 0;
}
.close-btn:hover { color: var(--text); background: var(--surface-2); }

/* Loading */
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
.spin-lg {
  display: inline-block;
  animation: spin 1s linear infinite;
  font-size: 22px;
}

.hint { font-size: 11px; font-family: 'SF Mono', 'Menlo', monospace; }

/* Results */
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
  background: var(--red-dim);
  color: var(--red);
  border: 1px solid rgba(248, 81, 73, 0.25);
}
.verdict-icon { font-size: 16px; }

.reasons {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 0;
}
.reasons li {
  font-size: 12px;
  color: var(--red);
  padding-left: 14px;
  position: relative;
}
.reasons li::before {
  content: '•';
  position: absolute;
  left: 4px;
}

.divider { height: 1px; background: var(--border); }

.section-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
  margin-bottom: -4px;
}

.field-grid { display: flex; flex-direction: column; gap: 8px; }

.field { display: flex; flex-direction: column; gap: 2px; }

.label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.value {
  font-size: 12px;
  color: var(--text);
  word-break: break-all;
}
.value.mono { font-family: 'SF Mono', 'Menlo', monospace; font-size: 11px; }
.value.small { font-size: 11px; }
.value.warn { color: var(--orange); }
.value.suspicious { color: var(--red); }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }
</style>
