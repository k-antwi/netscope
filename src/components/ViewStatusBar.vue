<script setup lang="ts">
defineProps<{
  total: number
  highlightCount: number
  highlightLabel: string
  highlightClass?: string
  lastRefreshText: string
}>()
</script>

<template>
  <footer class="statusbar">
    <span class="stat">
      <span class="stat-value">{{ total }}</span> connections
    </span>
    <span class="stat">
      <span class="stat-value" :class="highlightClass">{{ highlightCount }}</span>
      {{ highlightLabel }}
    </span>
    <span class="stat muted">{{ lastRefreshText }}</span>
  </footer>
</template>

<style scoped>
.statusbar {
  height: 32px;
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
.muted { margin-left: auto; }
</style>
