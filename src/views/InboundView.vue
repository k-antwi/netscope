<script setup lang="ts">
import { computed, ref } from 'vue'
import { useInbound } from '../composables/useInbound'
import { useInvestigation } from '../composables/useInvestigation'
import { useServiceInvestigation } from '../composables/useServiceInvestigation'
import InboundTable from '../components/InboundTable.vue'
import IpInspector from '../components/IpInspector.vue'
import ServiceInspector from '../components/ServiceInspector.vue'
import ViewToolbar from '../components/ViewToolbar.vue'
import ViewStatusBar from '../components/ViewStatusBar.vue'
import type { InboundConnection } from '../types'

const {
  connections,
  isLoading,
  lastRefreshText,
  showLocal,
  autoRefresh,
  refreshInterval,
  fetch,
} = useInbound()

const ipInvestigation = useInvestigation()
const serviceInvestigation = useServiceInvestigation()

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

// Which connection is currently selected (for row highlighting)
const selectedConnection = computed<InboundConnection | null>(
  () => serviceInvestigation.selectedConnection.value ?? (
    ipInvestigation.selectedIp.value
      ? (filtered.value.find(c => c.remote_ip === ipInvestigation.selectedIp.value) ?? null)
      : null
  )
)

function onSelect(c: InboundConnection) {
  if (c.state === 'LISTEN') {
    ipInvestigation.close()
    serviceInvestigation.investigate(c)
  } else if (c.remote_ip) {
    serviceInvestigation.close()
    ipInvestigation.investigate(c.remote_ip, c.remote_port)
  }
}

function closeAll() {
  ipInvestigation.close()
  serviceInvestigation.close()
}

const showIpPanel = computed(() => !!ipInvestigation.selectedIp.value)
const showServicePanel = computed(() => !!serviceInvestigation.selectedConnection.value)
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
        :selected-connection="selectedConnection"
        @select="onSelect"
      />

      <Transition name="panel">
        <IpInspector
          v-if="showIpPanel"
          :ip="ipInvestigation.selectedIp.value!"
          :port="ipInvestigation.selectedPort.value"
          :investigation="ipInvestigation.investigation.value"
          :is-loading="ipInvestigation.isInvestigating.value"
          @close="closeAll"
        />
        <ServiceInspector
          v-else-if="showServicePanel"
          :local-ip="serviceInvestigation.selectedConnection.value!.local_ip"
          :local-port="serviceInvestigation.selectedConnection.value!.local_port"
          :investigation="serviceInvestigation.investigation.value"
          :is-loading="serviceInvestigation.isInvestigating.value"
          @close="closeAll"
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
