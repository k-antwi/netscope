import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { IpInvestigation } from '../types'

export function useInvestigation() {
  const selectedIp = ref<string | null>(null)
  const selectedPort = ref(443)
  const investigation = ref<IpInvestigation | null>(null)
  const isInvestigating = ref(false)

  async function investigate(ip: string, port: number) {
    selectedIp.value = ip
    selectedPort.value = port
    investigation.value = null
    isInvestigating.value = true
    try {
      investigation.value = await invoke<IpInvestigation>('investigate_ip', { ip, port })
    } finally {
      isInvestigating.value = false
    }
  }

  function close() {
    selectedIp.value = null
    investigation.value = null
  }

  return { selectedIp, selectedPort, investigation, isInvestigating, investigate, close }
}
