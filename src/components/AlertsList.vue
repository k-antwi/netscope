<script setup lang="ts">
import type { Issue } from '../types'

defineProps<{
  grouped: { severity: Issue['severity']; items: Issue[] }[]
  isLoading: boolean
}>()

const SEV: Record<Issue['severity'], { label: string; icon: string; color: string; bg: string; border: string }> = {
  critical: {
    label: 'Critical',
    icon: '✕',
    color: 'var(--red)',
    bg: 'var(--red-dim)',
    border: 'rgba(248,81,73,0.35)',
  },
  high: {
    label: 'High',
    icon: '⚠',
    color: 'var(--orange)',
    bg: 'rgba(210,153,34,0.1)',
    border: 'rgba(210,153,34,0.35)',
  },
  warning: {
    label: 'Warning',
    icon: '!',
    color: '#e3b341',
    bg: 'rgba(227,179,65,0.08)',
    border: 'rgba(227,179,65,0.3)',
  },
  info: {
    label: 'Info',
    icon: 'i',
    color: 'var(--accent)',
    bg: 'rgba(88,166,255,0.07)',
    border: 'rgba(88,166,255,0.25)',
  },
}

const CATEGORY_LABEL: Record<string, string> = {
  'exposed-database': 'Exposed Database',
  'plaintext-service': 'Plaintext Service',
  'unencrypted-exposure': 'Unencrypted Exposure',
  'plaintext-outbound': 'Plaintext Outbound',
  'unusual-port': 'Unusual Port',
}
</script>

<template>
  <div class="list-wrap">
    <div v-if="isLoading && grouped.length === 0" class="empty">
      <span class="spin-lg">↺</span>
      <span>Scanning for issues…</span>
    </div>

    <div v-else-if="grouped.length === 0" class="empty clean">
      <span class="clean-icon">✓</span>
      <span class="clean-title">No issues detected</span>
      <span class="clean-sub">All services and connections look healthy</span>
    </div>

    <div v-else class="groups">
      <section v-for="group in grouped" :key="group.severity" class="group">
        <div
          class="group-header"
          :style="{ color: SEV[group.severity].color }"
        >
          <span class="group-icon">{{ SEV[group.severity].icon }}</span>
          {{ SEV[group.severity].label }}
          <span class="group-count">{{ group.items.length }}</span>
        </div>

        <div
          v-for="(issue, i) in group.items"
          :key="i"
          class="issue-card"
          :style="{
            borderLeftColor: SEV[group.severity].color,
            background: SEV[group.severity].bg,
            borderColor: SEV[group.severity].border,
          }"
        >
          <div class="issue-title">{{ issue.title }}</div>
          <div class="issue-detail">{{ issue.detail }}</div>
          <div class="issue-meta">
            <span class="chip process">{{ issue.process }}</span>
            <span class="chip pid">PID {{ issue.pid }}</span>
            <span v-if="issue.port" class="chip port">port {{ issue.port }}</span>
            <span v-if="issue.remote_ip" class="chip ip mono">{{ issue.remote_ip }}</span>
            <span class="chip cat">{{ CATEGORY_LABEL[issue.category] ?? issue.category }}</span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.list-wrap { flex: 1; overflow-y: auto; background: var(--bg); }

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 100%;
  color: var(--muted);
  font-size: 14px;
}

.empty.clean { gap: 6px; }
.clean-icon { font-size: 32px; color: var(--green); }
.clean-title { font-size: 15px; font-weight: 600; color: var(--text); }
.clean-sub { font-size: 12px; color: var(--muted); }

@keyframes spin { to { transform: rotate(360deg); } }
.spin-lg { display: inline-block; animation: spin 1s linear infinite; font-size: 24px; }

.groups { padding: 16px; display: flex; flex-direction: column; gap: 24px; }

.group { display: flex; flex-direction: column; gap: 8px; }

.group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}

.group-icon {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: currentColor;
  color: #000;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: 900;
  flex-shrink: 0;
}

.group-count {
  margin-left: auto;
  background: currentColor;
  color: #000;
  font-size: 10px;
  font-weight: 800;
  padding: 1px 6px;
  border-radius: 8px;
}

.issue-card {
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: 8px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.issue-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  line-height: 1.4;
}

.issue-detail {
  font-size: 12px;
  color: var(--muted);
  line-height: 1.5;
}

.issue-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 2px;
}

.chip {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 4px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--muted);
  white-space: nowrap;
}

.chip.process { color: var(--text); font-weight: 500; }
.chip.port { font-family: 'SF Mono', 'Menlo', monospace; }
.chip.ip { font-family: 'SF Mono', 'Menlo', monospace; }

.mono { font-family: 'SF Mono', 'Menlo', monospace; }
</style>
