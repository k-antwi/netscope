<script setup lang="ts">
import { ref, computed } from 'vue'
import type { InboundConnection } from '../types'
import { PORT_LABELS } from '../types'

const props = defineProps<{
  connections: InboundConnection[]
  isLoading: boolean
  selectedConnection: InboundConnection | null
  grouped: boolean
}>()

const emit = defineEmits<{
  select: [connection: InboundConnection]
}>()

type SortKey = 'process' | 'local_port' | 'local_ip' | 'state'
const sortKey = ref<SortKey>('state')
const sortAsc = ref(true)
const collapsed = ref(new Set<string>())

function sortItems(items: InboundConnection[]): InboundConnection[] {
  return [...items].sort((a, b) => {
    const av = a[sortKey.value]
    const bv = b[sortKey.value]
    const cmp = typeof av === 'number'
      ? (av as number) - (bv as number)
      : String(av).localeCompare(String(bv))
    return sortAsc.value ? cmp : -cmp
  })
}

const sorted = computed(() => sortItems(props.connections))

const groups = computed(() => {
  const map = new Map<string, InboundConnection[]>()
  for (const c of props.connections) {
    if (!map.has(c.process)) map.set(c.process, [])
    map.get(c.process)!.push(c)
  }
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([name, items]) => ({ name, items: sortItems(items) }))
})

function setSort(key: SortKey) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = true
  }
}

function toggleGroup(name: string) {
  const next = new Set(collapsed.value)
  next.has(name) ? next.delete(name) : next.add(name)
  collapsed.value = next
}

function portLabel(port: number): string {
  return PORT_LABELS[port] ?? String(port)
}

function sortIcon(key: SortKey): string {
  return sortKey.value === key ? (sortAsc.value ? ' ↑' : ' ↓') : ''
}

function isSelected(c: InboundConnection): boolean {
  const s = props.selectedConnection
  if (!s) return false
  if (c.state === 'LISTEN') {
    return s.state === 'LISTEN' && s.pid === c.pid && s.local_port === c.local_port
  }
  return s.state === 'ESTABLISHED' && s.remote_ip === c.remote_ip && s.remote_port === c.remote_port
}

function rowClass(c: InboundConnection): string {
  if (isSelected(c)) return 'row selected'
  if (c.state === 'ESTABLISHED') return 'row established'
  if (c.is_localhost_only) return 'row local'
  if (c.is_all_interfaces) return 'row exposed'
  return 'row'
}
</script>

<template>
  <div class="table-wrap">
    <div v-if="isLoading && connections.length === 0" class="empty">
      <span class="spin-lg">↺</span>
      <span>Scanning inbound traffic…</span>
    </div>

    <div v-else-if="connections.length === 0" class="empty">
      <span>No inbound connections found</span>
    </div>

    <table v-else>
      <thead>
        <tr>
          <th @click="setSort('process')" class="sortable">Process{{ sortIcon('process') }}</th>
          <th>PID</th>
          <th @click="setSort('local_ip')" class="sortable">Listening On{{ sortIcon('local_ip') }}</th>
          <th @click="setSort('local_port')" class="sortable">Port{{ sortIcon('local_port') }}</th>
          <th>Remote</th>
          <th @click="setSort('state')" class="sortable">State{{ sortIcon('state') }}</th>
          <th></th>
        </tr>
      </thead>

      <!-- Grouped mode -->
      <tbody v-if="grouped">
        <template v-for="group in groups" :key="group.name">
          <tr class="group-header" @click="toggleGroup(group.name)">
            <td colspan="7">
              <div class="group-header-content">
                <span class="chevron" :class="{ open: !collapsed.has(group.name) }">›</span>
                <span class="group-name">{{ group.name }}</span>
                <span class="group-count">{{ group.items.length }}</span>
              </div>
            </td>
          </tr>
          <template v-if="!collapsed.has(group.name)">
            <tr
              v-for="(c, i) in group.items"
              :key="i"
              :class="rowClass(c)"
              @click="emit('select', c)"
            >
              <td class="process muted-cell" :title="c.process">{{ c.process }}</td>
              <td class="pid mono">{{ c.pid }}</td>
              <td class="ip mono">
                <span
                  class="interface-badge"
                  :class="{ all: c.is_all_interfaces, local: c.is_localhost_only }"
                  :title="c.local_ip"
                >
                  {{ c.is_all_interfaces ? '*' : c.local_ip }}
                </span>
              </td>
              <td class="port">
                <span class="port-badge" :class="{ encrypted: c.is_encrypted }">
                  {{ portLabel(c.local_port) }}
                </span>
              </td>
              <td class="remote mono">
                <template v-if="c.remote_ip">{{ c.remote_ip }}:{{ c.remote_port }}</template>
                <span v-else class="muted">—</span>
              </td>
              <td class="state">
                <span class="state-badge" :class="c.state.toLowerCase()">{{ c.state }}</span>
              </td>
              <td class="action">
                <button
                  class="inspect-btn"
                  @click.stop="emit('select', c)"
                  :title="c.state === 'LISTEN' ? 'Inspect service' : 'Investigate remote IP'"
                >⌖</button>
              </td>
            </tr>
          </template>
        </template>
      </tbody>

      <!-- Flat mode -->
      <tbody v-else>
        <tr
          v-for="(c, i) in sorted"
          :key="i"
          :class="rowClass(c)"
          @click="emit('select', c)"
        >
          <td class="process" :title="c.process">{{ c.process }}</td>
          <td class="pid mono">{{ c.pid }}</td>
          <td class="ip mono">
            <span
              class="interface-badge"
              :class="{ all: c.is_all_interfaces, local: c.is_localhost_only }"
              :title="c.local_ip"
            >
              {{ c.is_all_interfaces ? '*' : c.local_ip }}
            </span>
          </td>
          <td class="port">
            <span class="port-badge" :class="{ encrypted: c.is_encrypted }">
              {{ portLabel(c.local_port) }}
            </span>
          </td>
          <td class="remote mono">
            <template v-if="c.remote_ip">{{ c.remote_ip }}:{{ c.remote_port }}</template>
            <span v-else class="muted">—</span>
          </td>
          <td class="state">
            <span class="state-badge" :class="c.state.toLowerCase()">{{ c.state }}</span>
          </td>
          <td class="action">
            <button
              class="inspect-btn"
              @click.stop="emit('select', c)"
              :title="c.state === 'LISTEN' ? 'Inspect service' : 'Investigate remote IP'"
            >⌖</button>
          </td>
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
  gap: 12px;
  height: 100%;
  color: var(--muted);
  font-size: 14px;
}

@keyframes spin { to { transform: rotate(360deg); } }
.spin-lg { display: inline-block; animation: spin 1s linear infinite; font-size: 24px; }

table { width: 100%; border-collapse: collapse; font-size: 12px; }
thead { position: sticky; top: 0; z-index: 1; background: var(--surface); }

th {
  padding: 8px 12px;
  text-align: left;
  font-weight: 500;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}
th.sortable { cursor: pointer; user-select: none; }
th.sortable:hover { color: var(--text); }

td {
  padding: 7px 12px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.5);
  white-space: nowrap;
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
  padding: 6px 12px;
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
}

.group-count {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 7px;
  border-radius: 10px;
  background: var(--border);
  color: var(--muted);
}

/* Row states */
.row { cursor: pointer; transition: background 0.1s; }
.row:hover { background: var(--surface-2); }
.row.exposed { background: rgba(210, 153, 34, 0.08); }
.row.exposed:hover { background: rgba(210, 153, 34, 0.14); }
.row.established { background: rgba(88, 166, 255, 0.08); }
.row.established:hover { background: rgba(88, 166, 255, 0.14); }
.row.local { opacity: 0.45; }
.row.local:hover { opacity: 0.7; background: var(--surface-2); }
.row.selected { background: rgba(88, 166, 255, 0.18) !important; outline: 1px solid var(--accent); }

.process { max-width: 120px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; }
.muted-cell { color: var(--muted); font-weight: 400; }
.pid { color: var(--muted); }
.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.remote { font-family: 'SF Mono', 'Menlo', monospace; font-size: 11px; color: var(--muted); }
.muted { color: var(--muted); }

.interface-badge {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--surface-2);
  border: 1px solid var(--border);
}
.interface-badge.all {
  color: var(--orange);
  background: rgba(210, 153, 34, 0.1);
  border-color: rgba(210, 153, 34, 0.3);
}
.interface-badge.local { color: var(--muted); }

.port-badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  background: var(--surface-2);
  color: var(--muted);
  border: 1px solid var(--border);
}
.port-badge.encrypted {
  background: var(--green-dim);
  color: var(--green);
  border-color: rgba(63, 185, 80, 0.3);
}

.state-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-weight: 600;
}
.state-badge.listen { background: rgba(210, 153, 34, 0.12); color: var(--orange); }
.state-badge.established { background: rgba(88, 166, 255, 0.12); color: var(--accent); }

.action { width: 36px; text-align: center; }
.inspect-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 16px;
  padding: 2px 4px;
  border-radius: 4px;
  line-height: 1;
}
.inspect-btn:hover { color: var(--accent); background: var(--surface-2); }
</style>
