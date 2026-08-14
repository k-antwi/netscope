<script setup lang="ts">
import { ref, computed } from 'vue'
import type { BrowserRequest, BrowserHeader } from '../types'

const props = defineProps<{
  requests: BrowserRequest[]
  selectedId: string | null
  grouped: boolean
}>()

const emit = defineEmits<{ select: [req: BrowserRequest] }>()

// ── Sorting ───────────────────────────────────────────────────────

type SortKey = 'method' | 'status' | 'browser' | 'domain' | 'path' | 'type' | 'size' | 'timingMs'

const sortKey = ref<SortKey | null>(null)
const sortAsc = ref(true)
const collapsed = ref(new Set<string>())

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

function getSortVal(req: BrowserRequest, key: SortKey): string | number {
  switch (key) {
    case 'method':   return req.method
    case 'status':   return req.status
    case 'browser':  return detectBrowser(req.requestHeaders)
    case 'domain':   return parseUrl(req.url).domain
    case 'path':     return parseUrl(req.url).path
    case 'type':     return contentType(req)
    case 'timingMs': return req.timingMs
    case 'size': {
      const cl = req.responseHeaders.find(h => h.name.toLowerCase() === 'content-length')
      return cl ? parseInt(cl.value, 10) || 0 : 0
    }
  }
}

function sortItems(items: BrowserRequest[]): BrowserRequest[] {
  if (!sortKey.value) return items
  const key = sortKey.value
  return [...items].sort((a, b) => {
    const va = getSortVal(a, key)
    const vb = getSortVal(b, key)
    const cmp = typeof va === 'number' && typeof vb === 'number'
      ? va - vb
      : String(va).localeCompare(String(vb))
    return sortAsc.value ? cmp : -cmp
  })
}

const sorted = computed(() => sortItems(props.requests))

const groups = computed(() => {
  const map = new Map<string, BrowserRequest[]>()
  for (const r of props.requests) {
    const domain = parseUrl(r.url).domain
    if (!map.has(domain)) map.set(domain, [])
    map.get(domain)!.push(r)
  }
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([domain, items]) => ({ domain, items: sortItems(items) }))
})

function toggleGroup(domain: string) {
  const next = new Set(collapsed.value)
  next.has(domain) ? next.delete(domain) : next.add(domain)
  collapsed.value = next
}

// ── Browser detection ─────────────────────────────────────────────

function detectBrowser(headers: BrowserHeader[]): string {
  const ua = headers.find(h => h.name.toLowerCase() === 'user-agent')?.value ?? ''
  if (!ua) return '—'
  if (/Edg\//.test(ua))          return 'Edge'
  if (/OPR\/|Opera/.test(ua))    return 'Opera'
  if (/Firefox\//.test(ua))      return 'Firefox'
  if (/Chrome\//.test(ua))       return 'Chrome'
  if (/Safari\//.test(ua))       return 'Safari'
  return '—'
}

const BROWSER_STYLE: Record<string, { color: string; bg: string }> = {
  Chrome:  { color: '#4fc3f7', bg: 'rgba(79,195,247,0.12)' },
  Firefox: { color: '#ff9a3c', bg: 'rgba(255,154,60,0.12)' },
  Edge:    { color: '#50b0f0', bg: 'rgba(80,176,240,0.12)' },
  Safari:  { color: '#a8d8a8', bg: 'rgba(168,216,168,0.12)' },
  Opera:   { color: '#f87171', bg: 'rgba(248,113,113,0.12)' },
}

function browserStyle(name: string): { color: string; bg: string } | null {
  return BROWSER_STYLE[name] ?? null
}

// ── URL / content helpers ─────────────────────────────────────────

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
  if (ct.includes('json'))       return 'XHR'
  if (ct.includes('html'))       return 'Doc'
  if (ct.includes('javascript')) return 'JS'
  if (ct.includes('css'))        return 'CSS'
  if (ct.includes('image/'))     return 'Img'
  if (ct.includes('font'))       return 'Font'
  if (ct.includes('xml'))        return 'XML'
  if (ct.includes('wasm'))       return 'Wasm'
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

// ── Row / badge classes ───────────────────────────────────────────

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
          <th class="col-method sortable" @click="setSort('method')">Method{{ sortIcon('method') }}</th>
          <th class="col-status sortable" @click="setSort('status')">Status{{ sortIcon('status') }}</th>
          <th class="col-browser sortable" @click="setSort('browser')">Browser{{ sortIcon('browser') }}</th>
          <th v-if="!grouped" class="col-domain sortable" @click="setSort('domain')">Domain{{ sortIcon('domain') }}</th>
          <th class="col-path sortable" @click="setSort('path')">Path{{ sortIcon('path') }}</th>
          <th class="col-type sortable" @click="setSort('type')">Type{{ sortIcon('type') }}</th>
          <th class="col-size sortable" @click="setSort('size')">Size{{ sortIcon('size') }}</th>
          <th class="col-time sortable" @click="setSort('timingMs')">Time{{ sortIcon('timingMs') }}</th>
        </tr>
      </thead>

      <!-- Grouped by domain -->
      <tbody v-if="grouped">
        <template v-for="group in groups" :key="group.domain">
          <tr class="group-header" @click="toggleGroup(group.domain)">
            <td :colspan="7">
              <div class="group-header-content">
                <span class="chevron" :class="{ open: !collapsed.has(group.domain) }">›</span>
                <span class="group-name">{{ group.domain }}</span>
                <span class="group-count">{{ group.items.length }}</span>
              </div>
            </td>
          </tr>
          <template v-if="!collapsed.has(group.domain)">
            <tr
              v-for="req in group.items"
              :key="req.id"
              class="row"
              :class="{ selected: req.id === selectedId, error: !!req.error }"
              @click="emit('select', req)"
            >
              <td class="col-method">
                <span class="method-badge" :class="methodClass(req.method)">{{ req.method }}</span>
              </td>
              <td class="col-status">
                <span class="status-badge" :class="statusClass(req.status, req.error)">
                  {{ req.error ? 'ERR' : req.status || '…' }}
                </span>
              </td>
              <td class="col-browser">
                <span
                  class="browser-badge"
                  :style="browserStyle(detectBrowser(req.requestHeaders))
                    ? { color: browserStyle(detectBrowser(req.requestHeaders))!.color, background: browserStyle(detectBrowser(req.requestHeaders))!.bg }
                    : {}"
                >{{ detectBrowser(req.requestHeaders) }}</span>
              </td>
              <td class="col-path mono" :title="parseUrl(req.url).path">
                {{ parseUrl(req.url).path }}
              </td>
              <td class="col-type">{{ contentType(req) }}</td>
              <td class="col-size">{{ formatSize(req) }}</td>
              <td class="col-time">{{ formatTime(req.timingMs) }}</td>
            </tr>
          </template>
        </template>
      </tbody>

      <!-- Flat mode -->
      <tbody v-else>
        <tr
          v-for="req in sorted"
          :key="req.id"
          class="row"
          :class="{ selected: req.id === selectedId, error: !!req.error }"
          @click="emit('select', req)"
        >
          <td class="col-method">
            <span class="method-badge" :class="methodClass(req.method)">{{ req.method }}</span>
          </td>
          <td class="col-status">
            <span class="status-badge" :class="statusClass(req.status, req.error)">
              {{ req.error ? 'ERR' : req.status || '…' }}
            </span>
          </td>
          <td class="col-browser">
            <span
              class="browser-badge"
              :style="browserStyle(detectBrowser(req.requestHeaders))
                ? { color: browserStyle(detectBrowser(req.requestHeaders))!.color, background: browserStyle(detectBrowser(req.requestHeaders))!.bg }
                : {}"
            >{{ detectBrowser(req.requestHeaders) }}</span>
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

th.sortable { cursor: pointer; user-select: none; }
th.sortable:hover { color: var(--text); }

td {
  padding: 5px 10px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Group header row */
.group-header {
  cursor: pointer;
  user-select: none;
}
.group-header td {
  padding: 0;
  border-bottom: 1px solid var(--border);
  background: var(--surface-2);
}
.group-header:hover td { filter: brightness(1.06); }

.group-header-content {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
}

.chevron {
  color: var(--muted);
  font-size: 13px;
  display: inline-block;
  transition: transform 0.15s;
  line-height: 1;
}
.chevron.open { transform: rotate(90deg); }

.group-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  font-family: 'SF Mono', 'Menlo', monospace;
}

.group-count {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 7px;
  border-radius: 10px;
  background: var(--border);
  color: var(--muted);
}

/* Column widths */
.col-method  { width: 68px; }
.col-status  { width: 56px; }
.col-browser { width: 72px; }
.col-domain  { width: 150px; }
.col-path    { /* flex */ }
.col-type    { width: 44px; text-align: center; color: var(--muted); font-size: 11px; }
.col-size    { width: 66px; text-align: right; color: var(--muted); }
.col-time    { width: 66px; text-align: right; color: var(--muted); }

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
.method-badge.get    { background: rgba(63,185,80,0.15);   color: var(--green); }
.method-badge.post   { background: rgba(88,166,255,0.15);  color: var(--accent); }
.method-badge.put    { background: rgba(210,153,34,0.15);  color: var(--orange); }
.method-badge.patch  { background: rgba(188,140,255,0.15); color: #bc8cff; }
.method-badge.delete { background: rgba(248,81,73,0.15);   color: var(--red); }
.method-badge.other  { background: var(--surface-2); color: var(--muted); }

/* Status badges */
.status-badge {
  font-size: 10px;
  font-weight: 600;
  font-family: 'SF Mono', 'Menlo', monospace;
  padding: 2px 5px;
  border-radius: 4px;
}
.status-badge.ok       { color: var(--green);  background: rgba(63,185,80,0.1); }
.status-badge.redirect { color: var(--accent); background: rgba(88,166,255,0.1); }
.status-badge.warn     { color: var(--orange); background: rgba(210,153,34,0.1); }
.status-badge.err      { color: var(--red);    background: rgba(248,81,73,0.1); }

/* Browser badge */
.browser-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--muted);
  background: var(--surface-2);
}
</style>
