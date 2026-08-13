import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { InboundConnection } from '../types'

export function useInbound() {
  const connections = ref<InboundConnection[]>([])
  const isLoading = ref(false)
  const lastRefresh = ref<Date | null>(null)
  const showLocal = ref(false)
  const autoRefresh = ref(true)
  const refreshInterval = ref(5)
  let timer: ReturnType<typeof setInterval> | null = null

  async function fetch() {
    isLoading.value = true
    try {
      connections.value = await invoke<InboundConnection[]>('get_inbound', {
        showLocal: showLocal.value,
      })
      lastRefresh.value = new Date()
    } finally {
      isLoading.value = false
    }
  }

  function startTimer() {
    stopTimer()
    if (autoRefresh.value) {
      timer = setInterval(fetch, refreshInterval.value * 1000)
    }
  }

  function stopTimer() {
    if (timer) { clearInterval(timer); timer = null }
  }

  const lastRefreshText = computed(() => {
    if (!lastRefresh.value) return 'never'
    const secs = Math.floor((Date.now() - lastRefresh.value.getTime()) / 1000)
    if (secs < 5) return 'just now'
    if (secs < 60) return `${secs}s ago`
    return `${Math.floor(secs / 60)}m ago`
  })

  watch([autoRefresh, refreshInterval], startTimer)
  watch(showLocal, fetch)
  onMounted(() => { fetch(); startTimer() })
  onUnmounted(stopTimer)

  return {
    connections,
    isLoading,
    lastRefreshText,
    showLocal,
    autoRefresh,
    refreshInterval,
    fetch,
  }
}
