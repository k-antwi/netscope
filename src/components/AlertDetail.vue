<script setup lang="ts">
import type { Issue } from '../types'
import { computed } from 'vue'
import { getRemediation } from '../data/issueRemediation'

const props = defineProps<{ issue: Issue }>()
defineEmits<{ close: [] }>()

const remediation = computed(() => getRemediation(props.issue))

const SEV_CONFIG = {
  critical: { label: 'Critical', color: 'var(--red)',    bg: 'var(--red-dim)',             border: 'rgba(248,81,73,0.3)' },
  high:     { label: 'High',     color: 'var(--orange)', bg: 'rgba(210,153,34,0.1)',       border: 'rgba(210,153,34,0.3)' },
  warning:  { label: 'Warning',  color: '#e3b341',       bg: 'rgba(227,179,65,0.08)',      border: 'rgba(227,179,65,0.28)' },
  info:     { label: 'Info',     color: 'var(--accent)', bg: 'rgba(88,166,255,0.07)',      border: 'rgba(88,166,255,0.25)' },
}

const sev = computed(() => SEV_CONFIG[props.issue.severity])
</script>

<template>
  <aside class="detail">
    <!-- Header -->
    <div class="detail-header">
      <div class="header-left">
        <span
          class="sev-badge"
          :style="{ color: sev.color, background: sev.bg, borderColor: sev.border }"
        >
          {{ sev.label }}
        </span>
        <div class="detail-title">{{ issue.title }}</div>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <div class="detail-body">

      <!-- Meta chips -->
      <div class="meta-row">
        <span class="chip process">{{ issue.process }}</span>
        <span class="chip pid">PID {{ issue.pid }}</span>
        <span v-if="issue.port" class="chip mono">port {{ issue.port }}</span>
        <span v-if="issue.remote_ip" class="chip mono">{{ issue.remote_ip }}</span>
      </div>

      <div class="divider" />

      <!-- Impact -->
      <div class="section">
        <div class="section-title impact-title">
          <span class="section-icon">⚡</span>
          Potential Impact
        </div>
        <ul class="impact-list">
          <li v-for="(item, i) in remediation.impact" :key="i">
            {{ item }}
          </li>
        </ul>
      </div>

      <div class="divider" />

      <!-- Fix steps -->
      <div class="section">
        <div class="section-title fix-title">
          <span class="section-icon">✓</span>
          How to Fix
        </div>
        <ol class="fix-list">
          <li v-for="(step, i) in remediation.steps" :key="i">
            <span class="step-num">{{ i + 1 }}</span>
            <span class="step-text">{{ step }}</span>
          </li>
        </ol>
      </div>

      <!-- Config hint -->
      <template v-if="remediation.configHint">
        <div class="divider" />
        <div class="section">
          <div class="section-title">
            <span class="section-icon">⌥</span>
            Config Example
          </div>
          <pre class="config-block">{{ remediation.configHint }}</pre>
        </div>
      </template>

    </div>
  </aside>
</template>

<style scoped>
.detail {
  width: var(--panel-w);
  flex-shrink: 0;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  gap: 10px;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.sev-badge {
  display: inline-flex;
  align-items: center;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 2px 8px;
  border-radius: 5px;
  border: 1px solid;
  align-self: flex-start;
}

.detail-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  line-height: 1.4;
}

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 14px;
  padding: 4px;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 2px;
}
.close-btn:hover { color: var(--text); background: var(--surface-2); }

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
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
.chip.mono { font-family: 'SF Mono', 'Menlo', monospace; }

.divider { height: 1px; background: var(--border); }

.section { display: flex; flex-direction: column; gap: 10px; }

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.impact-title { color: var(--red); }
.fix-title { color: var(--green); }

.section-icon { font-size: 12px; }

/* Impact bullets */
.impact-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  background: var(--red-dim);
  border: 1px solid rgba(248, 81, 73, 0.2);
  border-radius: 8px;
}

.impact-list li {
  font-size: 12px;
  color: var(--text);
  line-height: 1.55;
  padding-left: 14px;
  position: relative;
}

.impact-list li::before {
  content: '›';
  position: absolute;
  left: 0;
  color: var(--red);
  font-weight: 700;
}

/* Fix steps */
.fix-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  background: var(--green-dim);
  border: 1px solid rgba(63, 185, 80, 0.2);
  border-radius: 8px;
}

.fix-list li {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.step-num {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  background: var(--green);
  color: #000;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: 800;
  margin-top: 1px;
}

.step-text {
  font-size: 12px;
  color: var(--text);
  line-height: 1.55;
}

/* Config block */
.config-block {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.6;
}
</style>
