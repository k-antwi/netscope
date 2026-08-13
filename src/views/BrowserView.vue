<script setup lang="ts">
import { ref, computed } from 'vue'
import { useBrowser } from '../composables/useBrowser'
import RequestTable from '../components/RequestTable.vue'
import RequestDetail from '../components/RequestDetail.vue'
import type { BrowserRequest } from '../types'

const { requests, extensionConnected, totalBytes, clear } = useBrowser()

const search = ref('')
const selectedRequest = ref<BrowserRequest | null>(null)

const filtered = computed(() => {
  const q = search.value.toLowerCase()
  return q
    ? requests.value.filter(r => r.url.toLowerCase().includes(q) || r.method.toLowerCase().includes(q))
    : requests.value
})

function onSelect(req: BrowserRequest) {
  selectedRequest.value = selectedRequest.value?.id === req.id ? null : req
}

function onClear() {
  clear()
  selectedRequest.value = null
}

function formatBytes(bytes: number): string {
  if (!bytes) return '0 B'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
}
</script>

<template>
  <div class="view">
    <!-- Toolbar -->
    <div class="toolbar">
      <div class="search-wrap">
        <span class="search-icon">⌕</span>
        <input
          v-model="search"
          class="search-input"
          type="text"
          placeholder="Filter by URL or method…"
        />
        <button v-if="search" class="clear-search" @click="search = ''">✕</button>
      </div>

      <div class="toolbar-right">
        <div class="connection-indicator" :class="{ connected: extensionConnected }">
          <span class="conn-dot" />
          {{ extensionConnected ? 'Extension connected' : 'Waiting for extension' }}
        </div>
        <button class="clear-btn" @click="onClear" :disabled="requests.length === 0">
          Clear
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="content">
      <RequestTable
        :requests="filtered"
        :selected-id="selectedRequest?.id ?? null"
        @select="onSelect"
      />

      <Transition name="panel">
        <RequestDetail
          v-if="selectedRequest"
          :request="selectedRequest"
          @close="selectedRequest = null"
        />
      </Transition>
    </div>

    <!-- Status bar -->
    <div class="status-bar">
      <span>{{ filtered.length }} request{{ filtered.length !== 1 ? 's' : '' }}</span>
      <span v-if="totalBytes" class="transferred">{{ formatBytes(totalBytes) }} transferred</span>
    </div>
  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.search-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 7px;
  padding: 5px 10px;
}

.search-icon { color: var(--muted); font-size: 14px; }

.search-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: 12px;
  color: var(--text);
  font-family: inherit;
}

.search-input::placeholder { color: var(--muted); }

.clear-search {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  padding: 2px;
  line-height: 1;
}
.clear-search:hover { color: var(--text); }

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.connection-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--muted);
}

.conn-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--muted);
  transition: background 0.3s;
}

.connection-indicator.connected .conn-dot {
  background: var(--green);
  box-shadow: 0 0 6px rgba(63, 185, 80, 0.5);
}

.connection-indicator.connected { color: var(--green); }

.clear-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  padding: 4px 10px;
  transition: color 0.15s, border-color 0.15s;
}
.clear-btn:hover:not(:disabled) { color: var(--text); border-color: var(--muted); }
.clear-btn:disabled { opacity: 0.35; cursor: default; }

.content { display: flex; flex: 1; overflow: hidden; }

.status-bar {
  display: flex;
  justify-content: space-between;
  padding: 5px 14px;
  background: var(--surface);
  border-top: 1px solid var(--border);
  font-size: 11px;
  color: var(--muted);
  flex-shrink: 0;
}

.transferred { color: var(--accent); }

.panel-enter-active, .panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-enter-from, .panel-leave-to { transform: translateX(100%); opacity: 0; }
</style>
