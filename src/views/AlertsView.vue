<script setup lang="ts">
import { computed } from 'vue'
import { useAlerts } from '../composables/useAlerts'
import AlertsList from '../components/AlertsList.vue'

const { issues, isLoading, lastRefreshText, urgentCount, grouped, fetch } = useAlerts()

const criticalCount = computed(() => issues.value.filter(i => i.severity === 'critical').length)
const highCount = computed(() => issues.value.filter(i => i.severity === 'high').length)
const warningCount = computed(() => issues.value.filter(i => i.severity === 'warning').length)
const infoCount = computed(() => issues.value.filter(i => i.severity === 'info').length)
</script>

<template>
  <div class="view">
    <div class="alerts-toolbar">
      <div class="summary-chips">
        <span v-if="criticalCount" class="sev-chip critical">
          <span class="dot" />
          {{ criticalCount }} critical
        </span>
        <span v-if="highCount" class="sev-chip high">
          <span class="dot" />
          {{ highCount }} high
        </span>
        <span v-if="warningCount" class="sev-chip warning">
          <span class="dot" />
          {{ warningCount }} warning
        </span>
        <span v-if="infoCount" class="sev-chip info">
          <span class="dot" />
          {{ infoCount }} info
        </span>
        <span v-if="!isLoading && issues.length === 0" class="sev-chip clean">
          All clear
        </span>
      </div>

      <div class="toolbar-right">
        <span class="last-refresh">checked {{ lastRefreshText }}</span>
        <button class="refresh-btn" :class="{ spinning: isLoading }" @click="fetch" :disabled="isLoading">
          ↺
        </button>
      </div>
    </div>

    <AlertsList :grouped="grouped" :is-loading="isLoading" />

    <div class="status-bar">
      <span>{{ issues.length }} issue{{ issues.length !== 1 ? 's' : '' }} found</span>
      <span v-if="urgentCount" class="urgent-label">
        {{ urgentCount }} need{{ urgentCount !== 1 ? '' : 's' }} attention
      </span>
    </div>
  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

.alerts-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  gap: 12px;
}

.summary-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.sev-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 600;
  padding: 3px 9px;
  border-radius: 6px;
  border: 1px solid;
}

.sev-chip .dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.sev-chip.critical { color: var(--red); border-color: rgba(248,81,73,0.35); background: var(--red-dim); }
.sev-chip.high { color: var(--orange); border-color: rgba(210,153,34,0.35); background: rgba(210,153,34,0.1); }
.sev-chip.warning { color: #e3b341; border-color: rgba(227,179,65,0.3); background: rgba(227,179,65,0.08); }
.sev-chip.info { color: var(--accent); border-color: rgba(88,166,255,0.25); background: rgba(88,166,255,0.07); }
.sev-chip.clean { color: var(--green); border-color: rgba(63,185,80,0.3); background: var(--green-dim); }

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.last-refresh {
  font-size: 11px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.refresh-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--muted);
  cursor: pointer;
  font-size: 15px;
  padding: 3px 8px;
  line-height: 1;
  transition: color 0.15s;
}
.refresh-btn:hover:not(:disabled) { color: var(--text); border-color: var(--muted); }
.refresh-btn:disabled { opacity: 0.5; cursor: default; }

@keyframes spin { to { transform: rotate(360deg); } }
.refresh-btn.spinning { animation: spin 0.8s linear infinite; }

.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 16px;
  background: var(--surface);
  border-top: 1px solid var(--border);
  font-size: 11px;
  color: var(--muted);
  flex-shrink: 0;
}

.urgent-label { color: var(--orange); font-weight: 600; }
</style>
