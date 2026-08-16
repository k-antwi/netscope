<script setup lang="ts">
import { ref, computed } from 'vue'
import AppTabs from './components/AppTabs.vue'
import DashboardView from './views/DashboardView.vue'
import OutboundView from './views/OutboundView.vue'
import InboundView from './views/InboundView.vue'
import AlertsView from './views/AlertsView.vue'
import BrowserView from './views/BrowserView.vue'
import FileScanView from './views/FileScanView.vue'
import DefenderView from './views/DefenderView.vue'
import { useAlerts } from './composables/useAlerts'
import type { TabKey } from './components/AppTabs.vue'

const activeTab = ref<TabKey>('dashboard')
const { urgentCount } = useAlerts()
const badges = computed(() => ({ alerts: urgentCount.value }))
</script>

<template>
  <div class="app">
    <AppTabs :active-tab="activeTab" :badges="badges" @change="activeTab = $event" />
    <div class="app-content">
      <KeepAlive>
        <DashboardView v-if="activeTab === 'dashboard'" />
        <OutboundView v-else-if="activeTab === 'outbound'" />
        <InboundView v-else-if="activeTab === 'inbound'" />
        <AlertsView v-else-if="activeTab === 'alerts'" />
        <BrowserView v-else-if="activeTab === 'browser'" />
        <FileScanView v-else-if="activeTab === 'files'" />
        <DefenderView v-else-if="activeTab === 'defender'" />
      </KeepAlive>
    </div>
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

.app {
  display: flex;
  flex-direction: row;
  height: 100vh;
  overflow: hidden;
}

.app-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.https { color: var(--green) !important; }
.warn  { color: var(--orange) !important; }
</style>
