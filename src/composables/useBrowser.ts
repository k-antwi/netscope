import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { BrowserRequest } from '../types'

// Singleton state
const requests = ref<BrowserRequest[]>([])
const extensionConnected = ref(false)
let initialized = false

async function init() {
  if (initialized) return
  initialized = true

  await listen<BrowserRequest>('browser-request', event => {
    requests.value.unshift(event.payload)
    if (requests.value.length > 2000) {
      requests.value = requests.value.slice(0, 2000)
    }
  })

  await listen('extension-connected', () => { extensionConnected.value = true })
  await listen('extension-disconnected', () => { extensionConnected.value = false })
}

export function useBrowser() {
  init()

  const totalBytes = computed(() =>
    requests.value.reduce((sum, r) => {
      const cl = r.responseHeaders.find(h => h.name.toLowerCase() === 'content-length')
      return sum + (cl ? parseInt(cl.value, 10) || 0 : 0)
    }, 0)
  )

  function clear() { requests.value = [] }

  return { requests, extensionConnected, totalBytes, clear }
}
