<script setup lang="ts">
import { computed, ref } from 'vue'
import { useOutbound } from '../composables/useOutbound'
import { useInvestigation } from '../composables/useInvestigation'
import ConnectionTable from '../components/ConnectionTable.vue'
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
} = useOutbound()

const { selectedIp, selectedPort, investigation, isInvestigating, investigate, close } =
  useInvestigation()

const search = ref('')

const filtered = computed(() => {
  const q = search.value.toLowerCase()
  return q
    ? connections.value.filter(
        c => c.process.toLowerCase().includes(q) || c.remote_ip.includes(q)
      )
    : connections.value
})

const httpsCount = computed(() => filtered.value.filter(c => c.is_https).length)
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
      <ConnectionTable
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
      :highlight-count="httpsCount"
      highlight-label="HTTPS"
      highlight-class="https"
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
