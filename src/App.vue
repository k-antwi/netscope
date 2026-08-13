<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ConnectionTable from './components/ConnectionTable.vue'
import IpInspector from './components/IpInspector.vue'
import type { Connection, IpInvestigation } from './types'

const connections = ref<Connection[]>([])
const search = ref('')
const showLocal = ref(false)
const autoRefresh = ref(true)
const refreshInterval = ref(5)
const isLoading = ref(false)
const lastRefresh = ref<Date | null>(null)
const selectedIp = ref<string | null>(null)
const selectedPort = ref(443)
const investigation = ref<IpInvestigation | null>(null)
const isInvestigating = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

const filteredConnections = computed(() => {
  const q = search.value.toLowerCase()
  return connections.value.filter(c =>
    !q || c.process.toLowerCase().includes(q) || c.remote_ip.includes(q)
  )
})

const httpsCount = computed(() =>
  filteredConnections.value.filter(c => c.is_https).length
)

const lastRefreshText = computed(() => {
  if (!lastRefresh.value) return 'never'
  const secs = Math.floor((Date.now() - lastRefresh.value.getTime()) / 1000)
  if (secs < 5) return 'just now'
  if (secs < 60) return `${secs}s ago`
  return `${Math.floor(secs / 60)}m ago`
})

async function fetchConnections() {
  isLoading.value = true
  try {
    connections.value = await invoke<Connection[]>('get_connections', {
      showLocal: showLocal.value,
    })
    lastRefresh.value = new Date()
  } finally {
    isLoading.value = false
  }
}

async function handleInvestigate(ip: string, port: number) {
  selectedIp.value = ip
  selectedPort.value = port
  investigation.value = null
  isInvestigating.value = true
  try {
    investigation.value = await invoke<IpInvestigation>('investigate_ip', {
      ip,
      port,
    })
  } finally {
    isInvestigating.value = false
  }
}

function closeInspector() {
  selectedIp.value = null
  investigation.value = null
}

function startTimer() {
  stopTimer()
  if (autoRefresh.value) {
    timer = setInterval(fetchConnections, refreshInterval.value * 1000)
  }
}

function stopTimer() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
}

watch([autoRefresh, refreshInterval], startTimer)
watch(showLocal, fetchConnections)

onMounted(() => {
  fetchConnections()
  startTimer()
})

onUnmounted(stopTimer)
</script>

<template>
  <div class="app">
    <header class="toolbar" data-tauri-drag-region>
      <div class="brand">
        <span class="brand-icon">⬡</span>
        <span class="brand-name">NetScope</span>
      </div>
      <div class="controls">
        <input
          v-model="search"
          class="search"
          placeholder="Filter process or IP…"
          spellcheck="false"
        />
        <label class="toggle-label">
          <input type="checkbox" v-model="showLocal" />
          <span>Local</span>
        </label>
        <label class="toggle-label">
          <input type="checkbox" v-model="autoRefresh" />
          <span>Auto</span>
        </label>
        <select v-model.number="refreshInterval" class="interval-select" :disabled="!autoRefresh">
          <option :value="3">3s</option>
          <option :value="5">5s</option>
          <option :value="10">10s</option>
          <option :value="30">30s</option>
        </select>
        <button class="refresh-btn" @click="fetchConnections" :disabled="isLoading">
          <span :class="{ spin: isLoading }">↺</span>
        </button>
      </div>
    </header>

    <main class="main">
      <ConnectionTable
        :connections="filteredConnections"
        :is-loading="isLoading"
        :selected-ip="selectedIp"
        @investigate="handleInvestigate"
      />
      <Transition name="panel">
        <IpInspector
          v-if="selectedIp"
          :ip="selectedIp"
          :port="selectedPort"
          :investigation="investigation"
          :is-loading="isInvestigating"
          @close="closeInspector"
        />
      </Transition>
    </main>

    <footer class="statusbar">
      <span class="stat">
        <span class="stat-value">{{ filteredConnections.length }}</span> connections
      </span>
      <span class="stat">
        <span class="stat-value https">{{ httpsCount }}</span> HTTPS
      </span>
      <span class="stat muted">{{ lastRefreshText }}</span>
    </footer>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg: #0d1117;
  --surface: #161b22;
  --surface-2: #21262d;
  --border: #30363d;
  --text: #c9d1d9;
  --muted: #8b949e;
  --accent: #58a6ff;
  --green: #3fb950;
  --green-dim: rgba(63, 185, 80, 0.12);
  --red: #f85149;
  --red-dim: rgba(248, 81, 73, 0.12);
  --orange: #d29922;
  --toolbar-h: 48px;
  --statusbar-h: 32px;
  --panel-w: 360px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 13px;
}

body { background: var(--bg); color: var(--text); }

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

/* Toolbar */
.toolbar {
  height: var(--toolbar-h);
  padding: 0 16px 0 80px; /* 80px left = space for macOS traffic lights */
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  user-select: none;
}

.brand {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  letter-spacing: 0.05em;
  color: var(--accent);
  margin-right: 8px;
}

.brand-icon { font-size: 18px; }

.controls {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}

.search {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 10px;
  color: var(--text);
  font-size: 12px;
  width: 220px;
  outline: none;
}
.search:focus { border-color: var(--accent); }
.search::placeholder { color: var(--muted); }

.toggle-label {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--muted);
  font-size: 12px;
  cursor: pointer;
  user-select: none;
}
.toggle-label input { accent-color: var(--accent); cursor: pointer; }

.interval-select {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 6px;
  color: var(--text);
  font-size: 12px;
  outline: none;
  cursor: pointer;
}
.interval-select:disabled { opacity: 0.4; cursor: not-allowed; }

.refresh-btn {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 10px;
  color: var(--text);
  font-size: 16px;
  cursor: pointer;
  line-height: 1;
}
.refresh-btn:hover { border-color: var(--accent); color: var(--accent); }
.refresh-btn:disabled { opacity: 0.4; cursor: not-allowed; }

@keyframes spin { to { transform: rotate(360deg); } }
.spin { display: inline-block; animation: spin 0.8s linear infinite; }

/* Main layout */
.main {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Status bar */
.statusbar {
  height: var(--statusbar-h);
  padding: 0 16px;
  display: flex;
  align-items: center;
  gap: 20px;
  background: var(--surface);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  font-size: 11px;
  color: var(--muted);
}

.stat { display: flex; gap: 4px; }
.stat-value { color: var(--text); font-variant-numeric: tabular-nums; }
.stat-value.https { color: var(--green); }
.muted { color: var(--muted); margin-left: auto; }

/* Panel transition */
.panel-enter-active, .panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-enter-from, .panel-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
