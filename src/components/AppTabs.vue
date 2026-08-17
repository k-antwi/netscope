<script setup lang="ts">
export type TabKey = 'dashboard' | 'outbound' | 'inbound' | 'alerts' | 'browser' | 'files' | 'defender'

defineProps<{
  activeTab: TabKey
  badges?: Partial<Record<TabKey, number>>
}>()

defineEmits<{ change: [tab: TabKey] }>()

const tabs: { key: TabKey; label: string }[] = [
  { key: 'dashboard', label: 'Dashboard' },
  { key: 'outbound', label: 'Outbound' },
  { key: 'inbound',  label: 'Inbound' },
  { key: 'alerts',   label: 'Alerts' },
  { key: 'browser',  label: 'Browser' },
  { key: 'files',    label: 'File Scan' },
  { key: 'defender', label: 'Defender' },
]
</script>

<template>
  <nav class="sidebar" data-tauri-drag-region>
    <!-- Traffic light clearance zone (macOS overlay title bar) -->
    <div class="traffic-zone" />

    <!-- Brand -->
    <div class="brand">
      <svg class="brand-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="9" />
        <circle cx="12" cy="12" r="3.5" />
        <line x1="3" y1="12" x2="8.5" y2="12" />
        <line x1="15.5" y1="12" x2="21" y2="12" />
        <line x1="12" y1="3" x2="12" y2="8.5" />
        <line x1="12" y1="15.5" x2="12" y2="21" />
      </svg>
      <span class="brand-name">NetScope</span>
    </div>

    <div class="nav-divider" />

    <!-- Nav items -->
    <div class="nav-list">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="nav-item"
        :class="{ active: activeTab === tab.key }"
        @click="$emit('change', tab.key)"
      >
        <!-- Dashboard -->
        <svg v-if="tab.key === 'dashboard'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="3" width="7" height="7" rx="1"/>
          <rect x="14" y="3" width="7" height="7" rx="1"/>
          <rect x="3" y="14" width="7" height="7" rx="1"/>
          <rect x="14" y="14" width="7" height="7" rx="1"/>
        </svg>

        <!-- Outbound -->
        <svg v-else-if="tab.key === 'outbound'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M7 17L17 7" />
          <path d="M17 7H10" />
          <path d="M17 7v7" />
          <path d="M3 21h4" />
          <path d="M3 17h2" />
          <path d="M3 13h1" />
        </svg>

        <!-- Inbound -->
        <svg v-else-if="tab.key === 'inbound'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17 7L7 17" />
          <path d="M7 17h7" />
          <path d="M7 17v-7" />
          <path d="M21 3h-4" />
          <path d="M21 7h-2" />
          <path d="M21 11h-1" />
        </svg>

        <!-- Alerts -->
        <svg v-else-if="tab.key === 'alerts'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>

        <!-- Browser -->
        <svg v-else-if="tab.key === 'browser'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <line x1="2" y1="12" x2="22" y2="12" />
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>

        <!-- File Scan -->
        <svg v-else-if="tab.key === 'files'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <circle cx="10" cy="14" r="2" />
          <path d="m12.5 16.5 1.5 1.5" />
        </svg>

        <!-- Defender -->
        <svg v-else-if="tab.key === 'defender'" class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2L3 7v5c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-9-5z" />
          <polyline points="9 12 11 14 15 10" />
        </svg>

        <span class="nav-label">{{ tab.label }}</span>

        <span v-if="(badges?.[tab.key] ?? 0) > 0" class="badge">
          {{ badges?.[tab.key] }}
        </span>
      </button>
    </div>

    <!-- Version footer -->
    <div class="sidebar-footer">
      <span class="footer-label">v0.1.0</span>
    </div>
  </nav>
</template>

<style scoped>
.sidebar {
  width: 200px;
  flex-shrink: 0;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-right: 1px solid var(--border);
  user-select: none;
  -webkit-app-region: drag;
  overflow: hidden;
}

/* Interactive elements must opt out of the drag region */
.sidebar button {
  -webkit-app-region: no-drag;
}

.traffic-zone {
  height: 32px;
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 16px 14px;
  flex-shrink: 0;
}

.brand-icon {
  width: 22px;
  height: 22px;
  color: var(--accent);
  flex-shrink: 0;
}

.brand-name {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: var(--text);
}

.nav-divider {
  height: 1px;
  background: var(--border);
  margin: 0 16px 8px;
  flex-shrink: 0;
}

.nav-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 8px;
  overflow: hidden;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 10px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s, color 0.12s;
  position: relative;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text);
}

.nav-item.active {
  background: rgba(88, 166, 255, 0.1);
  color: var(--text);
}

.nav-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 6px;
  bottom: 6px;
  width: 3px;
  background: var(--accent);
  border-radius: 0 2px 2px 0;
}

.nav-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  transition: color 0.12s;
}

.nav-item.active .nav-icon {
  color: var(--accent);
}

.nav-label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  background: var(--red);
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 8px;
  line-height: 1.5;
  min-width: 16px;
  text-align: center;
  flex-shrink: 0;
}

.sidebar-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.footer-label {
  font-size: 10px;
  color: var(--muted);
  opacity: 0.6;
  font-variant-numeric: tabular-nums;
}
</style>
