<script setup lang="ts">
import { computed, ref } from 'vue'
import { useInbound } from '../composables/useInbound'
import { useInvestigation } from '../composables/useInvestigation'
import InboundTable from '../components/InboundTable.vue'
import IpInspector from '../components/IpInspector.vue'
import ViewToolbar from '../components/ViewToolbar.vue'
import ViewStatusBar from '../components/ViewStatusBar.vue'

const {
  connections,
  isLoading,
  lastRefreshText,
  showLocal,
  autoRefresh,
  refreshInterval,
  fetch,
} = useInbound()

const { selectedIp, selectedPort, investigation, isInvestigating, investigate, close } =
  useInvestigation()

const search = ref('')

const filtered = computed(() => {
  const q = search.value.toLowerCase()
  return q
    ? connections.value.filter(
        c =>
          c.process.toLowerCase().includes(q) ||
          c.remote_ip.includes(q) ||
          String(c.local_port).includes(q)
      )
    : connections.value
})

const exposedCount = computed(
  () => filtered.value.filter(c => c.state === 'LISTEN' && c.is_all_interfaces).length
)
</script>

<template>
  <div class="view">
    <ViewToolbar
      v-model:search="search"
      v-model:show-local="showLocal"
      v-model:auto-refresh="autoRefresh"
      v-model:refresh-interval="refreshInterval"
      :is-loading="isLoading"
      @refresh="fetch"
    />

    <div class="content">
      <InboundTable
        :connections="filtered"
        :is-loading="isLoading"
        :selected-ip="selectedIp"
        @investigate="investigate"
      />
      <Transition name="panel">
        <IpInspector
          v-if="selectedIp"
          :ip="selectedIp"
          :port="selectedPort"
          :investigation="investigation"
          :is-loading="isInvestigating"
          @close="close"
        />
      </Transition>
    </div>

    <ViewStatusBar
      :total="filtered.length"
      :highlight-count="exposedCount"
      highlight-label="exposed externally"
      highlight-class="warn"
      :last-refresh-text="lastRefreshText"
    />
  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; flex: 1; overflow: hidden; }
.content { display: flex; flex: 1; overflow: hidden; }

.panel-enter-active, .panel-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-enter-from, .panel-leave-to { transform: translateX(100%); opacity: 0; }
</style>
