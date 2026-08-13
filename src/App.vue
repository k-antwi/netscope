<script setup lang="ts">
import { ref, computed } from 'vue'
import AppTabs from './components/AppTabs.vue'
import OutboundView from './views/OutboundView.vue'
import InboundView from './views/InboundView.vue'
import AlertsView from './views/AlertsView.vue'
import { useAlerts } from './composables/useAlerts'
import type { TabKey } from './components/AppTabs.vue'

const activeTab = ref<TabKey>('outbound')

const { urgentCount } = useAlerts()
const badges = computed(() => ({ alerts: urgentCount.value }))
</script>

<template>
  <div class="app">
    <header class="app-header" data-tauri-drag-region>
      <div class="brand">
        <span class="brand-icon">⬡</span>
        <span class="brand-name">NetScope</span>
      </div>
      <AppTabs :active-tab="activeTab" :badges="badges" @change="activeTab = $event" />
    </header>

    <KeepAlive>
      <OutboundView v-if="activeTab === 'outbound'" />
      <InboundView v-else-if="activeTab === 'inbound'" />
      <AlertsView v-else-if="activeTab === 'alerts'" />
    </KeepAlive>
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
  --panel-w: 360px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 13px;
}

body { background: var(--bg); color: var(--text); }

.app { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }

.app-header {
  height: 48px;
  padding: 0 16px 0 80px;
  display: flex;
  align-items: center;
  gap: 16px;
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
}
.brand-icon { font-size: 18px; }

/* shared stat colours used by status bars */
.https { color: var(--green) !important; }
.warn  { color: var(--orange) !important; }
</style>
