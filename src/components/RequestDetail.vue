<script setup lang="ts">
import { ref, computed } from 'vue'
import type { BrowserRequest, BrowserHeader } from '../types'

const props = defineProps<{ request: BrowserRequest }>()
defineEmits<{ close: [] }>()

type Tab = 'overview' | 'request' | 'response'
const activeTab = ref<Tab>('overview')
const copied = ref<string | null>(null)

// ── URL / time helpers ───────────────────────────────────────────

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

function statusClass(status: number, error: string | null): string {
  if (error || !status) return 'err'
  if (status < 300) return 'ok'
  if (status < 400) return 'redirect'
  if (status < 500) return 'warn'
  return 'err'
}

// ── Body formatting ──────────────────────────────────────────────

type BodyFormat = 'json' | 'form' | 'raw'
interface FormattedBody { text: string; format: BodyFormat }

function formatBody(body: string | null): FormattedBody | null {
  if (!body) return null
  const cleaned = body.replace(/^﻿/, '').trim()

  try {
    return { text: JSON.stringify(JSON.parse(cleaned), null, 2), format: 'json' }
  } catch { /* try next */ }

  if (/^[\w%+.-]+=/.test(cleaned) && cleaned.includes('&')) {
    try {
      const obj: Record<string, string> = {}
      new URLSearchParams(cleaned).forEach((v, k) => { obj[k] = v })
      return { text: JSON.stringify(obj, null, 2), format: 'form' }
    } catch { /* fall through */ }
  }

  return { text: body, format: 'raw' }
}

const requestBody = computed(() => formatBody(props.request.requestBody))

// ── Copy helpers ─────────────────────────────────────────────────

async function copyToClipboard(text: string, key: string) {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = key
    setTimeout(() => { copied.value = null }, 1800)
  } catch { /* clipboard unavailable */ }
}

function headersToObject(headers: BrowserHeader[]): Record<string, string> {
  const obj: Record<string, string> = {}
  for (const h of headers) obj[h.name] = h.value
  return obj
}

function tryParseJson(s: string | null): unknown {
  if (!s) return null
  try { return JSON.parse(s.replace(/^﻿/, '').trim()) } catch { return s }
}

function copyFullRequest() {
  const payload = {
    method: props.request.method,
    url: props.request.url,
    status: props.request.status,
    statusText: props.request.statusText,
    timingMs: props.request.timingMs,
    fromCache: props.request.fromCache,
    timestamp: new Date(props.request.timestamp).toISOString(),
    initiator: props.request.initiator || undefined,
    tabUrl: props.request.tabUrl || undefined,
    requestHeaders: headersToObject(props.request.requestHeaders),
    responseHeaders: headersToObject(props.request.responseHeaders),
    requestBody: tryParseJson(props.request.requestBody),
    error: props.request.error || undefined,
  }
  copyToClipboard(JSON.stringify(payload, null, 2), 'full')
}

function copyRequestHeaders() {
  copyToClipboard(JSON.stringify(headersToObject(props.request.requestHeaders), null, 2), 'reqHeaders')
}

function copyResponseHeaders() {
  copyToClipboard(JSON.stringify(headersToObject(props.request.responseHeaders), null, 2), 'resHeaders')
}

function copyBody() {
  if (requestBody.value) copyToClipboard(requestBody.value.text, 'body')
}
</script>

<template>
  <aside class="detail">

    <!-- Header -->
    <div class="detail-header">
      <div class="header-url" :title="request.url">
        <span class="method-tag">{{ request.method }}</span>
        <span class="url-text mono">{{ request.url }}</span>
      </div>
      <div class="header-actions">
        <button
          class="icon-btn"
          :class="{ done: copied === 'full' }"
          title="Copy full request as JSON"
          @click="copyFullRequest"
        >
          <!-- checkmark when copied -->
          <svg v-if="copied === 'full'" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          <!-- copy icon -->
          <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        </button>
        <button class="close-btn" @click="$emit('close')">✕</button>
      </div>
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

      <!-- REQUEST TAB -->
      <template v-else-if="activeTab === 'request'">

        <!-- Request headers -->
        <div class="section-label">
          Request Headers
          <button
            class="copy-label-btn"
            :class="{ done: copied === 'reqHeaders' }"
            title="Copy as JSON"
            @click="copyRequestHeaders"
          >
            <svg v-if="copied === 'reqHeaders'" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <svg v-else width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2"/>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
            {{ copied === 'reqHeaders' ? 'Copied' : 'Copy' }}
          </button>
        </div>

        <div v-if="request.requestHeaders.length" class="headers-table">
          <div v-for="h in request.requestHeaders" :key="h.name" class="header-row">
            <span class="header-name">{{ h.name }}</span>
            <span class="header-value mono">{{ h.value }}</span>
          </div>
        </div>
        <div v-else class="empty-section">No request headers captured</div>

        <!-- Request body -->
        <template v-if="requestBody">
          <div class="divider" />
          <div class="section-label">
            Request Body
            <span class="format-tag" :class="requestBody.format">{{ requestBody.format.toUpperCase() }}</span>
            <button
              class="copy-label-btn"
              :class="{ done: copied === 'body' }"
              title="Copy body"
              @click="copyBody"
            >
              <svg v-if="copied === 'body'" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              <svg v-else width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
              {{ copied === 'body' ? 'Copied' : 'Copy' }}
            </button>
          </div>
          <pre class="body-block"><code>{{ requestBody.text }}</code></pre>
        </template>
      </template>

      <!-- RESPONSE TAB -->
      <template v-else-if="activeTab === 'response'">

        <!-- Response headers -->
        <div class="section-label">
          Response Headers
          <button
            class="copy-label-btn"
            :class="{ done: copied === 'resHeaders' }"
            title="Copy as JSON"
            @click="copyResponseHeaders"
          >
            <svg v-if="copied === 'resHeaders'" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <svg v-else width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2"/>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
            {{ copied === 'resHeaders' ? 'Copied' : 'Copy' }}
          </button>
        </div>

        <div v-if="request.responseHeaders.length" class="headers-table">
          <div v-for="h in request.responseHeaders" :key="h.name" class="header-row">
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

/* Icon button in header (copy / close) */
.icon-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--muted);
  cursor: pointer;
  padding: 4px 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s, border-color 0.15s, background 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--muted); }
.icon-btn.done { color: var(--green); border-color: rgba(63,185,80,0.4); }

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 14px;
  padding: 4px 6px;
  border-radius: 4px;
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
.value.ok       { color: var(--green); }
.value.redirect { color: var(--accent); }
.value.warn     { color: var(--orange); }
.value.err      { color: var(--red); }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.divider { height: 1px; background: var(--border); }

/* Section label row */
.section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.format-tag {
  font-size: 9px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.format-tag.json { background: rgba(88,166,255,0.15); color: var(--accent); }
.format-tag.form { background: rgba(63,185,80,0.15);  color: var(--green); }
.format-tag.raw  { background: var(--surface-2);      color: var(--muted); }

/* Inline copy button inside section-label */
.copy-label-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
  background: none;
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--muted);
  cursor: pointer;
  font-size: 9px;
  font-weight: 600;
  padding: 2px 6px;
  text-transform: none;
  letter-spacing: 0;
  transition: color 0.15s, border-color 0.15s;
  white-space: nowrap;
}
.copy-label-btn:hover { color: var(--text); border-color: var(--muted); }
.copy-label-btn.done { color: var(--green); border-color: rgba(63,185,80,0.4); }

/* Headers table */
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

/* Body code block */
.body-block {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  white-space: pre;
  overflow-x: auto;
  overflow-y: auto;
  max-height: 320px;
  line-height: 1.6;
  tab-size: 2;
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
