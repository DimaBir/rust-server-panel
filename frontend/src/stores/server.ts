import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '../services/api'
import type { GameServer, CreateServerRequest } from '../types'

export const useServerStore = defineStore('server', () => {
  const servers = ref<GameServer[]>([])
  const loading = ref(false)

  async function fetchServers() {
    loading.value = true
    try {
      const res = await api.get<GameServer[]>('/servers')
      servers.value = res.data
    } catch {
      servers.value = []
    } finally {
      loading.value = false
    }
  }

  async function createServer(req: CreateServerRequest): Promise<{ id: string } | null> {
    try {
      const res = await api.post<{ id: string; name: string; status: string }>('/servers', req)
      await fetchServers()
      return { id: res.data.id }
    } catch {
      return null
    }
  }

  async function deleteServer(id: string): Promise<boolean> {
    try {
      await api.delete(`/servers/${id}`)
      await fetchServers()
      return true
    } catch {
      return false
    }
  }

  function getServer(id: string): GameServer | undefined {
    return servers.value.find((s) => s.id === id)
  }

  return {
    servers,
    loading,
    fetchServers,
    createServer,
    deleteServer,
    getServer,
  }
})
