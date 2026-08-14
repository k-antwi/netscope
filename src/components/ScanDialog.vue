<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

const props = defineProps<{
  open: boolean
  initial: string
}>()

const emit = defineEmits<{
  search: [name: string]
  close: []
}>()

const name = ref('')
const inputEl = ref<HTMLInputElement | null>(null)

watch(
  () => props.open,
  async (isOpen) => {
    if (!isOpen) return
    name.value = props.initial
    await nextTick()
    inputEl.value?.focus()
    inputEl.value?.select()
  }
)

function submit() {
  const trimmed = name.value.trim()
  if (!trimmed) {
    inputEl.value?.focus()
    return
  }
  emit('search', trimmed)
}
</script>

<template>
  <Transition name="dialog">
    <div v-if="open" class="overlay" @click.self="emit('close')">
      <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="scan-dialog-title">
        <div class="dialog-header">
          <div>
            <div id="scan-dialog-title" class="dialog-title">Scan for a file</div>
            <div class="dialog-sub">Searches every readable folder on this computer</div>
          </div>
          <button class="close-btn" @click="emit('close')" title="Close">✕</button>
        </div>

        <form class="dialog-body" @submit.prevent="submit">
          <label class="field-label" for="scan-file-name">File name</label>
          <input
            id="scan-file-name"
            ref="inputEl"
            v-model="name"
            class="name-input"
            placeholder="e.g. passwords.txt"
            spellcheck="false"
            autocomplete="off"
            @keydown.esc.prevent="emit('close')"
          />
          <div class="hint">
            Partial names match too. Use <code>*</code> and <code>?</code> for wildcards —
            <code>*.pdf</code>, <code>report-?.csv</code>.
          </div>

          <div class="dialog-actions">
            <button type="button" class="btn ghost" @click="emit('close')">Cancel</button>
            <button type="submit" class="btn primary" :disabled="!name.trim()">Search</button>
          </div>
        </form>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(1, 4, 9, 0.65);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.dialog {
  width: 440px;
  max-width: calc(100vw - 32px);
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55);
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}

.dialog-title { font-size: 14px; font-weight: 600; color: var(--text); }
.dialog-sub { font-size: 11px; color: var(--muted); margin-top: 2px; }

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 13px;
  padding: 4px;
  border-radius: 4px;
  line-height: 1;
  flex-shrink: 0;
}
.close-btn:hover { color: var(--text); background: var(--surface-2); }

.dialog-body { padding: 16px; display: flex; flex-direction: column; gap: 8px; }

.field-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.name-input {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 9px 11px;
  color: var(--text);
  font-size: 13px;
  font-family: 'SF Mono', 'Menlo', monospace;
  outline: none;
  width: 100%;
}
.name-input:focus { border-color: var(--accent); }
.name-input::placeholder { color: var(--muted); font-family: inherit; }

.hint { font-size: 11px; color: var(--muted); line-height: 1.5; }
.hint code {
  font-family: 'SF Mono', 'Menlo', monospace;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 0 3px;
  color: var(--text);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}

.btn {
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  cursor: pointer;
  border: 1px solid var(--border);
}
.btn.ghost { background: var(--surface-2); color: var(--text); }
.btn.ghost:hover { border-color: var(--muted); }
.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #0d1117;
  font-weight: 600;
}
.btn.primary:hover { filter: brightness(1.08); }
.btn.primary:disabled { opacity: 0.4; cursor: not-allowed; filter: none; }

.dialog-enter-active, .dialog-leave-active { transition: opacity 0.15s ease; }
.dialog-enter-active .dialog, .dialog-leave-active .dialog { transition: transform 0.15s ease; }
.dialog-enter-from, .dialog-leave-to { opacity: 0; }
.dialog-enter-from .dialog, .dialog-leave-to .dialog { transform: translateY(-8px) scale(0.98); }
</style>
