<script setup lang="ts">
defineProps<{
  search: string
  showLocal: boolean
  autoRefresh: boolean
  refreshInterval: number
  isLoading: boolean
}>()

defineEmits<{
  'update:search': [v: string]
  'update:showLocal': [v: boolean]
  'update:autoRefresh': [v: boolean]
  'update:refreshInterval': [v: number]
  refresh: []
}>()
</script>

<template>
  <div class="toolbar">
    <input
      class="search"
      :value="search"
      placeholder="Filter process or IP…"
      spellcheck="false"
      @input="$emit('update:search', ($event.target as HTMLInputElement).value)"
    />

    <div class="controls">
      <label class="toggle-label">
        <input
          type="checkbox"
          :checked="showLocal"
          @change="$emit('update:showLocal', ($event.target as HTMLInputElement).checked)"
        />
        <span>Local</span>
      </label>

      <label class="toggle-label">
        <input
          type="checkbox"
          :checked="autoRefresh"
          @change="$emit('update:autoRefresh', ($event.target as HTMLInputElement).checked)"
        />
        <span>Auto</span>
      </label>

      <select
        class="interval-select"
        :value="refreshInterval"
        :disabled="!autoRefresh"
        @change="$emit('update:refreshInterval', Number(($event.target as HTMLSelectElement).value))"
      >
        <option :value="3">3s</option>
        <option :value="5">5s</option>
        <option :value="10">10s</option>
        <option :value="30">30s</option>
      </select>

      <button class="refresh-btn" :disabled="isLoading" @click="$emit('refresh')">
        <span :class="{ spin: isLoading }">↺</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.search {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 5px 10px;
  color: var(--text);
  font-size: 12px;
  width: 220px;
  outline: none;
}
.search:focus { border-color: var(--accent); }
.search::placeholder { color: var(--muted); }

.controls { display: flex; align-items: center; gap: 8px; margin-left: auto; }

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
</style>
