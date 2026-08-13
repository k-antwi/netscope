import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Issue } from '../types'

// Singleton state — shared across all callers
const issues = ref<Issue[]>([])
const isLoading = ref(false)
const lastRefreshText = ref('—')
let initialized = false

async function doFetch() {
  isLoading.value = true
  try {
    issues.value = await invoke<Issue[]>('get_issues')
    lastRefreshText.value = new Date().toLocaleTimeString()
  } catch {
    // keep previous state on error
  } finally {
    isLoading.value = false
  }
}

export function useAlerts() {
  if (!initialized) {
    initialized = true
    doFetch()
    setInterval(doFetch, 15_000)
  }

  const urgentCount = computed(
    () => issues.value.filter(i => i.severity === 'critical' || i.severity === 'high').length
  )

  const grouped = computed(() => {
    const order: Issue['severity'][] = ['critical', 'high', 'warning', 'info']
    return order
      .map(sev => ({ severity: sev, items: issues.value.filter(i => i.severity === sev) }))
      .filter(g => g.items.length > 0)
  })

  return { issues, isLoading, lastRefreshText, urgentCount, grouped, fetch: doFetch }
}
