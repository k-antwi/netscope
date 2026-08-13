<script setup lang="ts">
import type { BrowserRequest } from '../types'

const props = defineProps<{
  requests: BrowserRequest[]
  selectedId: string | null
}>()

const emit = defineEmits<{ select: [req: BrowserRequest] }>()

// ── Formatters ───────────────────────────────────────────────────

function parseUrl(url: string): { domain: string; path: string } {
  try {
    const u = new URL(url)
    return { domain: u.hostname, path: u.pathname + u.search }
  } catch {
    return { domain: url, path: '' }
  }
}

function contentType(req: BrowserRequest): string {
  const ct = req.responseHeaders.find(h => h.name.toLowerCase() === 'content-type')?.value ?? ''
  if (ct.includes('json')) return 'XHR'
  if (ct.includes('html')) return 'Doc'
  if (ct.includes('javascript')) return 'JS'
  if (ct.includes('css')) return 'CSS'
  if (ct.includes('image/')) return 'Img'
  if (ct.includes('font')) return 'Font'
  if (ct.includes('xml')) return 'XML'
  if (ct.includes('wasm')) return 'Wasm'
  return req.error ? 'Err' : '—'
}

function formatTime(ms: number): string {
  if (ms <= 0) return '—'
  if (ms < 1000) return `${Math.round(ms)} ms`
  return `${(ms / 1000).toFixed(2)} s`
}

function formatSize(req: BrowserRequest): string {
  const cl = req.responseHeaders.find(h => h.name.toLowerCase() === 'content-length')
  const bytes = cl ? parseInt(cl.value, 10) : 0
  if (!bytes) return req.fromCache ? 'cache' : '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

// ── CSS class helpers ─────────────────────────────────────────────

function methodClass(method: string): string {
  return ({ GET: 'get', POST: 'post', PUT: 'put', PATCH: 'patch', DELETE: 'delete' }[method] ?? 'other')
}

function statusClass(status: number, error: string | null): string {
  if (error || status === 0) return 'err'
  if (status < 300) return 'ok'
  if (status < 400) return 'redirect'
  if (status < 500) return 'warn'
  return 'err'
}
</script>

<template>
  <div class="table-wrap">
    <div v-if="requests.length === 0" class="empty">
      <span class="empty-icon">⌁</span>
      <span class="empty-title">No requests captured</span>
      <span class="empty-sub">Install the NetScope extension and browse to see traffic here</span>
    </div>

    <table v-else>
      <thead>
        <tr>
          <th class="col-method">Method</th>
          <th class="col-status">Status</th>
          <th class="col-domain">Domain</th>
          <th class="col-path">Path</th>
          <th class="col-type">Type</th>
          <th class="col-size">Size</th>
          <th class="col-time">Time</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="req in requests"
          :key="req.id"
          class="row"
          :class="{ selected: req.id === selectedId, error: !!req.error }"
          @click="emit('select', req)"
        >
          <td class="col-method">
            <span class="method-badge" :class="methodClass(req.method)">{{ req.method }}</span>
          </td>
          <td class="col-status">
            <span
              class="status-badge"
              :class="statusClass(req.status, req.error)"
            >
              {{ req.error ? 'ERR' : req.status || '…' }}
            </span>
          </td>
          <td class="col-domain mono" :title="req.url">
            {{ parseUrl(req.url).domain }}
          </td>
          <td class="col-path mono" :title="parseUrl(req.url).path">
            {{ parseUrl(req.url).path }}
          </td>
          <td class="col-type">{{ contentType(req) }}</td>
          <td class="col-size">{{ formatSize(req) }}</td>
          <td class="col-time">{{ formatTime(req.timingMs) }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrap { flex: 1; overflow-y: auto; background: var(--bg); }

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  color: var(--muted);
}

.empty-icon { font-size: 36px; opacity: 0.4; }
.empty-title { font-size: 14px; font-weight: 500; color: var(--text); }
.empty-sub { font-size: 12px; color: var(--muted); }

table { width: 100%; border-collapse: collapse; font-size: 12px; table-layout: fixed; }
thead { position: sticky; top: 0; z-index: 1; background: var(--surface); }

th {
  padding: 7px 10px;
  text-align: left;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
  overflow: hidden;
}

td {
  padding: 5px 10px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Column widths */
.col-method { width: 70px; }
.col-status { width: 58px; }
.col-domain { width: 160px; }
.col-path   { /* flex */ }
.col-type   { width: 46px; text-align: center; color: var(--muted); font-size: 11px; }
.col-size   { width: 68px; text-align: right; color: var(--muted); }
.col-time   { width: 68px; text-align: right; color: var(--muted); }

.row { cursor: pointer; transition: background 0.08s; }
.row:hover { background: var(--surface-2); }
.row.selected { background: rgba(88, 166, 255, 0.12) !important; outline: 1px solid var(--accent); }
.row.error td { color: var(--red); opacity: 0.8; }

.mono { font-family: 'SF Mono', 'Menlo', monospace; font-size: 11px; }

/* Method badges */
.method-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 5px;
  border-radius: 4px;
  letter-spacing: 0.04em;
}
.method-badge.get    { background: rgba(63,185,80,0.15);  color: var(--green); }
.method-badge.post   { background: rgba(88,166,255,0.15); color: var(--accent); }
.method-badge.put    { background: rgba(210,153,34,0.15); color: var(--orange); }
.method-badge.patch  { background: rgba(188,140,255,0.15); color: #bc8cff; }
.method-badge.delete { background: rgba(248,81,73,0.15);  color: var(--red); }
.method-badge.other  { background: var(--surface-2); color: var(--muted); }

/* Status badges */
.status-badge {
  font-size: 10px;
  font-weight: 600;
  font-family: 'SF Mono', 'Menlo', monospace;
  padding: 2px 5px;
  border-radius: 4px;
}
.status-badge.ok       { color: var(--green); background: rgba(63,185,80,0.1); }
.status-badge.redirect { color: var(--accent); background: rgba(88,166,255,0.1); }
.status-badge.warn     { color: var(--orange); background: rgba(210,153,34,0.1); }
.status-badge.err      { color: var(--red); background: rgba(248,81,73,0.1); }
</style>
