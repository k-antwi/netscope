<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Connection } from '../types'
import { PORT_LABELS } from '../types'

const props = defineProps<{
  connections: Connection[]
  isLoading: boolean
  selectedIp: string | null
}>()

const emit = defineEmits<{
  investigate: [ip: string, port: number]
}>()

type SortKey = 'process' | 'remote_ip' | 'remote_port' | 'state'
const sortKey = ref<SortKey>('process')
const sortAsc = ref(true)

const sorted = computed(() => {
  return [...props.connections].sort((a, b) => {
    const av = a[sortKey.value]
    const bv = b[sortKey.value]
    const cmp = typeof av === 'number' ? (av as number) - (bv as number)
      : String(av).localeCompare(String(bv))
    return sortAsc.value ? cmp : -cmp
  })
})

function setSort(key: SortKey) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = true
  }
}

function portLabel(port: number): string {
  return PORT_LABELS[port] ?? String(port)
}

function rowClass(c: Connection): string {
  if (c.remote_ip === props.selectedIp) return 'row selected'
  if (c.is_https) return 'row https'
  if (c.remote_port === 22) return 'row ssh'
  if (c.is_local) return 'row local'
  return 'row'
}

function sortIcon(key: SortKey): string {
  if (sortKey.value !== key) return ''
  return sortAsc.value ? ' ↑' : ' ↓'
}
</script>

<template>
  <div class="table-wrap">
    <div v-if="isLoading && connections.length === 0" class="empty">
      <span class="spin-lg">↺</span>
      <span>Scanning connections…</span>
    </div>

    <div v-else-if="connections.length === 0" class="empty">
      <span>No connections found</span>
    </div>

    <table v-else>
      <thead>
        <tr>
          <th @click="setSort('process')" class="sortable">
            Process{{ sortIcon('process') }}
          </th>
          <th>PID</th>
          <th @click="setSort('remote_ip')" class="sortable">
            Remote IP{{ sortIcon('remote_ip') }}
          </th>
          <th @click="setSort('remote_port')" class="sortable">
            Port{{ sortIcon('remote_port') }}
          </th>
          <th @click="setSort('state')" class="sortable">
            State{{ sortIcon('state') }}
          </th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(c, i) in sorted"
          :key="i"
          :class="rowClass(c)"
          @click="emit('investigate', c.remote_ip, c.remote_port)"
        >
          <td class="process" :title="c.process">{{ c.process }}</td>
          <td class="pid mono">{{ c.pid }}</td>
          <td class="ip mono">{{ c.remote_ip }}</td>
          <td class="port">
            <span class="port-badge" :class="{ https: c.is_https, ssh: c.remote_port === 22 }">
              {{ portLabel(c.remote_port) }}
            </span>
          </td>
          <td class="state">{{ c.state }}</td>
          <td class="action">
            <button
              v-if="!c.is_local"
              class="investigate-btn"
              @click.stop="emit('investigate', c.remote_ip, c.remote_port)"
              title="Investigate IP"
            >
              ⌖
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrap {
  flex: 1;
  overflow-y: auto;
  background: var(--bg);
}

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
.spin-lg {
  display: inline-block;
  animation: spin 1s linear infinite;
  font-size: 24px;
}

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

th.sortable {
  cursor: pointer;
  user-select: none;
}
th.sortable:hover { color: var(--text); }

td {
  padding: 7px 12px;
  border-bottom: 1px solid rgba(48, 54, 61, 0.5);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row {
  cursor: pointer;
  transition: background 0.1s;
}
.row:hover { background: var(--surface-2); }
.row.https { background: var(--green-dim); }
.row.https:hover { background: rgba(63, 185, 80, 0.18); }
.row.selected { background: rgba(88, 166, 255, 0.12); outline: 1px solid var(--accent); }
.row.local { opacity: 0.5; }
.row.ssh { background: rgba(210, 153, 34, 0.08); }

.process { max-width: 120px; font-weight: 500; }
.pid { color: var(--muted); }
.ip { font-family: 'SF Mono', 'Menlo', monospace; color: var(--text); }
.mono { font-family: 'SF Mono', 'Menlo', monospace; }

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
.port-badge.https {
  background: var(--green-dim);
  color: var(--green);
  border-color: rgba(63, 185, 80, 0.3);
}
.port-badge.ssh {
  background: rgba(210, 153, 34, 0.12);
  color: var(--orange);
  border-color: rgba(210, 153, 34, 0.3);
}

.state { color: var(--muted); font-size: 11px; }

.action { width: 36px; text-align: center; }
.investigate-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 16px;
  padding: 2px 4px;
  border-radius: 4px;
  line-height: 1;
}
.investigate-btn:hover { color: var(--accent); background: var(--surface-2); }
</style>
