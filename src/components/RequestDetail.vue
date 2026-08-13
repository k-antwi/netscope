<script setup lang="ts">
import { ref, computed } from 'vue'
import type { BrowserRequest } from '../types'

const props = defineProps<{ request: BrowserRequest }>()
defineEmits<{ close: [] }>()

type Tab = 'overview' | 'request' | 'response'
const activeTab = ref<Tab>('overview')

const parsedUrl = computed(() => {
  try { return new URL(props.request.url) } catch { return null }
})

function formatTime(ms: number): string {
  if (ms <= 0) return '—'
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(3)} s`
}

function formatDate(ts: number): string {
  return new Date(ts).toLocaleTimeString()
}

function formatBody(body: string | null): string {
  if (!body) return ''
  try { return JSON.stringify(JSON.parse(body), null, 2) } catch { return body }
}

const requestBody = computed(() => formatBody(props.request.requestBody))

function statusClass(status: number, error: string | null): string {
  if (error || !status) return 'err'
  if (status < 300) return 'ok'
  if (status < 400) return 'redirect'
  if (status < 500) return 'warn'
  return 'err'
}
</script>

<template>
  <aside class="detail">
    <div class="detail-header">
      <div class="header-url" :title="request.url">
        <span class="method-tag">{{ request.method }}</span>
        <span class="url-text mono">{{ request.url }}</span>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <!-- Tab bar -->
    <div class="tab-bar">
      <button
        v-for="t in (['overview', 'request', 'response'] as Tab[])"
        :key="t"
        class="tab-btn"
        :class="{ active: activeTab === t }"
        @click="activeTab = t"
      >
        {{ t.charAt(0).toUpperCase() + t.slice(1) }}
      </button>
    </div>

    <div class="detail-body">

      <!-- OVERVIEW -->
      <template v-if="activeTab === 'overview'">
        <div class="field-group">
          <div class="field">
            <span class="label">Status</span>
            <span class="value" :class="statusClass(request.status, request.error)">
              <template v-if="request.error">{{ request.error }}</template>
              <template v-else>{{ request.status }} {{ request.statusText }}</template>
            </span>
          </div>
          <div class="field">
            <span class="label">Method</span>
            <span class="value mono">{{ request.method }}</span>
          </div>
          <div class="field">
            <span class="label">Duration</span>
            <span class="value mono">{{ formatTime(request.timingMs) }}</span>
          </div>
          <div class="field">
            <span class="label">Cached</span>
            <span class="value">{{ request.fromCache ? 'Yes' : 'No' }}</span>
          </div>
          <div class="field">
            <span class="label">Time</span>
            <span class="value mono">{{ formatDate(request.timestamp) }}</span>
          </div>
        </div>

        <div class="divider" />

        <div class="field-group">
          <div class="field">
            <span class="label">Host</span>
            <span class="value mono">{{ parsedUrl?.hostname ?? '—' }}</span>
          </div>
          <div class="field">
            <span class="label">Path</span>
            <span class="value mono small">{{ parsedUrl?.pathname ?? '—' }}</span>
          </div>
          <div v-if="parsedUrl?.search" class="field">
            <span class="label">Query</span>
            <span class="value mono small">{{ parsedUrl.search }}</span>
          </div>
          <div v-if="parsedUrl?.protocol" class="field">
            <span class="label">Protocol</span>
            <span class="value mono">{{ parsedUrl.protocol.replace(':', '') }}</span>
          </div>
        </div>

        <template v-if="request.initiator || request.tabUrl">
          <div class="divider" />
          <div class="field-group">
            <div v-if="request.tabUrl" class="field">
              <span class="label">Initiated from</span>
              <span class="value mono small">{{ request.tabUrl }}</span>
            </div>
            <div v-if="request.initiator" class="field">
              <span class="label">Initiator</span>
              <span class="value mono small">{{ request.initiator }}</span>
            </div>
          </div>
        </template>
      </template>

      <!-- REQUEST -->
      <template v-else-if="activeTab === 'request'">
        <div class="section-label">Request Headers</div>
        <div v-if="request.requestHeaders.length" class="headers-table">
          <div
            v-for="h in request.requestHeaders"
            :key="h.name"
            class="header-row"
          >
            <span class="header-name">{{ h.name }}</span>
            <span class="header-value mono">{{ h.value }}</span>
          </div>
        </div>
        <div v-else class="empty-section">No request headers captured</div>

        <template v-if="requestBody">
          <div class="divider" />
          <div class="section-label">Request Body</div>
          <pre class="body-block">{{ requestBody }}</pre>
        </template>
      </template>

      <!-- RESPONSE -->
      <template v-else-if="activeTab === 'response'">
        <div class="section-label">Response Headers</div>
        <div v-if="request.responseHeaders.length" class="headers-table">
          <div
            v-for="h in request.responseHeaders"
            :key="h.name"
            class="header-row"
          >
            <span class="header-name">{{ h.name }}</span>
            <span class="header-value mono">{{ h.value }}</span>
          </div>
        </div>
        <div v-else class="empty-section">No response headers captured</div>

        <div class="divider" />
        <div class="notice">
          Response body is not available without SSL proxying.
        </div>
      </template>

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
  gap: 8px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.header-url {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  flex: 1;
}

.method-tag {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--muted);
  letter-spacing: 0.06em;
}

.url-text {
  font-size: 11px;
  color: var(--text);
  word-break: break-all;
  line-height: 1.4;
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

/* Tab bar */
.tab-bar {
  display: flex;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.tab-btn {
  flex: 1;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 8px 0;
  font-size: 11px;
  font-weight: 500;
  color: var(--muted);
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.tab-btn:hover { color: var(--text); }
.tab-btn.active { color: var(--accent); border-bottom-color: var(--accent); }

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field-group { display: flex; flex-direction: column; gap: 8px; }

.field { display: flex; flex-direction: column; gap: 2px; }

.label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.value { font-size: 12px; color: var(--text); word-break: break-all; }
.value.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.value.small { font-size: 11px; }
.value.ok { color: var(--green); }
.value.redirect { color: var(--accent); }
.value.warn { color: var(--orange); }
.value.err { color: var(--red); }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.divider { height: 1px; background: var(--border); }

.section-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.headers-table {
  display: flex;
  flex-direction: column;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}

.header-row {
  display: flex;
  gap: 8px;
  padding: 5px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  align-items: baseline;
}
.header-row:last-child { border-bottom: none; }

.header-name {
  color: var(--accent);
  font-weight: 500;
  flex-shrink: 0;
  min-width: 100px;
  max-width: 130px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-value {
  color: var(--text);
  word-break: break-all;
  line-height: 1.4;
}

.body-block {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.5;
  max-height: 300px;
  overflow-y: auto;
}

.empty-section {
  font-size: 12px;
  color: var(--muted);
  padding: 8px 0;
}

.notice {
  font-size: 11px;
  color: var(--muted);
  padding: 8px 10px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
}
</style>
