import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { InboundConnection, ServiceInvestigation } from '../types'

export function useServiceInvestigation() {
  const selectedConnection = ref<InboundConnection | null>(null)
  const investigation = ref<ServiceInvestigation | null>(null)
  const isInvestigating = ref(false)

  async function investigate(conn: InboundConnection) {
    selectedConnection.value = conn
    investigation.value = null
    isInvestigating.value = true
    try {
      investigation.value = await invoke<ServiceInvestigation>('investigate_service', {
        pid: conn.pid,
        localPort: conn.local_port,
        localIp: conn.local_ip,
      })
    } finally {
      isInvestigating.value = false
    }
  }

  function close() {
    selectedConnection.value = null
    investigation.value = null
  }

  return { selectedConnection, investigation, isInvestigating, investigate, close }
}
