<script setup lang="ts">
export type TabKey = 'outbound' | 'inbound' | 'alerts'

defineProps<{
  activeTab: TabKey
  badges?: Partial<Record<TabKey, number>>
}>()

defineEmits<{ change: [tab: TabKey] }>()

const tabs: { key: TabKey; label: string; icon: string }[] = [
  { key: 'outbound', label: 'Outbound', icon: '↗' },
  { key: 'inbound',  label: 'Inbound',  icon: '↙' },
  { key: 'alerts',   label: 'Alerts',   icon: '⚑' },
]
</script>

<template>
  <nav class="tabs">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      class="tab"
      :class="{ active: activeTab === tab.key, 'has-badge': (badges?.[tab.key] ?? 0) > 0 }"
      @click="$emit('change', tab.key)"
    >
      <span class="tab-icon">{{ tab.icon }}</span>
      {{ tab.label }}
      <span v-if="(badges?.[tab.key] ?? 0) > 0" class="badge">{{ badges![tab.key] }}</span>
    </button>
  </nav>
</template>

<style scoped>
.tabs {
  display: flex;
  gap: 2px;
  background: var(--surface-2);
  border-radius: 8px;
  padding: 3px;
}

.tab {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 14px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}

.tab:hover { color: var(--text); }

.tab.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.tab-icon { font-size: 13px; }

.badge {
  background: var(--red);
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 8px;
  line-height: 1.4;
  min-width: 16px;
  text-align: center;
}
</style>
