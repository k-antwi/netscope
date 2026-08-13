import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import type { Issue } from '../types'

// Singleton state — shared across all callers
const issues = ref<Issue[]>([])
const isLoading = ref(false)
const lastRefreshText = ref('—')
let initialized = false
let firstFetch = true
const seenKeys = new Set<string>()

function issueKey(i: Issue): string {
  return `${i.category}|${i.process}|${i.pid}|${i.port}`
}

function severityPrefix(sev: Issue['severity']): string {
  switch (sev) {
    case 'critical': return '🔴 Critical'
    case 'high':     return '🟠 High'
    case 'warning':  return '🟡 Warning'
    default:         return 'ℹ️ Info'
  }
}

async function notify(issue: Issue) {
  try {
    let granted = await isPermissionGranted()
    if (!granted) {
      const permission = await requestPermission()
      granted = permission === 'granted'
    }
    if (granted) {
      sendNotification({
        title: `${severityPrefix(issue.severity)}: ${issue.title}`,
        body: issue.detail,
      })
    }
  } catch { /* notifications unavailable */ }
}

async function doFetch() {
  isLoading.value = true
  try {
    const fresh = await invoke<Issue[]>('get_issues')

    if (firstFetch) {
      // Populate seen set silently — don't notify for issues already present at startup
      firstFetch = false
      for (const i of fresh) seenKeys.add(issueKey(i))
    } else {
      for (const i of fresh) {
        const k = issueKey(i)
        if (!seenKeys.has(k)) {
          seenKeys.add(k)
          notify(i)
        }
      }
    }

    issues.value = fresh
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
