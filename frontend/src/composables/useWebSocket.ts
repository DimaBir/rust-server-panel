import { ref, onUnmounted } from 'vue'

export function useWebSocket(serverId: string, channel: string = 'console') {
  const messages = ref<string[]>([])
  const connected = ref(false)
  const error = ref<string | null>(null)

  let ws: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let intentionalClose = false

  function connect() {
    if (ws && ws.readyState === WebSocket.OPEN) return

    const token = localStorage.getItem('jwt_token')
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${window.location.host}/ws/${serverId}/${channel}${token ? `?token=${token}` : ''}`

    ws = new WebSocket(wsUrl)

    ws.onopen = () => {
      connected.value = true
      error.value = null
    }

    ws.onmessage = (event: MessageEvent) => {
      messages.value.push(String(event.data))
    }

    ws.onerror = () => {
      error.value = 'WebSocket error'
    }

    ws.onclose = () => {
      connected.value = false
      if (!intentionalClose) {
        reconnectTimer = setTimeout(connect, 3000)
      }
    }
  }

  function send(message: string) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(message)
    }
  }

  function disconnect() {
    intentionalClose = true
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (ws) {
      ws.close()
      ws = null
    }
    connected.value = false
  }

  function clearMessages() {
    messages.value = []
  }

  onUnmounted(() => {
    disconnect()
  })

  return {
    messages,
    connected,
    error,
    connect,
    send,
    disconnect,
    clearMessages,
  }
}
