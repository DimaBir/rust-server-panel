import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../services/api'
import type { GameServer } from '../types'

export const useServerStore = defineStore('server', () => {
  const servers = ref<GameServer[]>([])
  const activeServerId = ref<string | null>(localStorage.getItem('active_server_id'))
  const loading = ref(false)

  const activeServer = computed(() =>
    servers.value.find((s) => s.id === activeServerId.value) ?? servers.value[0] ?? null
  )

  async function fetchServers() {
    loading.value = true
    try {
      const res = await api.get<GameServer[]>('/servers')
      servers.value = res.data
      // Auto-select first server if none saved or saved no longer exists
      if (!activeServerId.value || !servers.value.find((s) => s.id === activeServerId.value)) {
        const first = servers.value[0]
        if (first) {
          setActiveServer(first.id)
        }
      }
    } catch {
      servers.value = []
    } finally {
      loading.value = false
    }
  }

  function setActiveServer(id: string) {
    activeServerId.value = id
    localStorage.setItem('active_server_id', id)
  }

  return {
    servers,
    activeServerId,
    activeServer,
    loading,
    fetchServers,
    setActiveServer,
  }
})
