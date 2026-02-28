<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import { useServerStore } from '../stores/server'

const serverStore = useServerStore()
const terminalRef = ref<HTMLElement | null>(null)
const commandInput = ref('')
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

const activeServerId = computed(() => serverStore.activeServerId ?? '')

const quickCommands = [
  { label: 'status', cmd: 'status' },
  { label: 'serverinfo', cmd: 'serverinfo' },
  { label: 'players', cmd: 'global.playerlist' },
  { label: 'save', cmd: 'server.save' },
]

function initTerminal() {
  if (!terminalRef.value) return

  terminal = new Terminal({
    theme: {
      background: '#0a0a0b',
      foreground: '#33ff33',
      cursor: '#33ff33',
      selectionBackground: 'rgba(51, 255, 51, 0.3)',
    },
    fontFamily: '"Cascadia Code", "Fira Code", monospace',
    fontSize: 13,
    cursorBlink: true,
    convertEol: true,
    scrollback: 5000,
    disableStdin: true,
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(terminalRef.value)
  fitAddon.fit()

  terminal.writeln('\x1b[1;34m=== Server Console ===\x1b[0m')
  terminal.writeln('\x1b[90mConnecting...\x1b[0m')
  terminal.writeln('')
}

function connectWebSocket() {
  if (!activeServerId.value) return
  // Close existing
  if (ws) {
    ws.onclose = null
    ws.close()
  }
  if (reconnectTimer) clearTimeout(reconnectTimer)

  const token = localStorage.getItem('jwt_token')
  const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const url = `${wsProtocol}//${window.location.host}/ws/${activeServerId.value}/console${token ? '?token=' + token : ''}`

  ws = new WebSocket(url)

  ws.onopen = () => {
    terminal?.writeln('\x1b[32mConnected.\x1b[0m')
  }

  ws.onmessage = (event: MessageEvent) => {
    terminal?.writeln(String(event.data))
  }

  ws.onerror = () => {
    terminal?.writeln('\x1b[31mWebSocket error.\x1b[0m')
  }

  ws.onclose = () => {
    terminal?.writeln('\x1b[33mDisconnected. Reconnecting in 3s...\x1b[0m')
    reconnectTimer = setTimeout(connectWebSocket, 3000)
  }
}

function sendCommand(cmd?: string) {
  const command = cmd ?? commandInput.value.trim()
  if (!command) return

  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(command)
    terminal?.writeln(`\x1b[1;37m> ${command}\x1b[0m`)
  } else {
    terminal?.writeln('\x1b[31mNot connected.\x1b[0m')
  }

  if (!cmd) {
    commandHistory.value.unshift(command)
    if (commandHistory.value.length > 50) commandHistory.value.pop()
    historyIndex.value = -1
    commandInput.value = ''
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (historyIndex.value < commandHistory.value.length - 1) {
      historyIndex.value++
      commandInput.value = commandHistory.value[historyIndex.value] ?? ''
    }
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (historyIndex.value > 0) {
      historyIndex.value--
      commandInput.value = commandHistory.value[historyIndex.value] ?? ''
    } else {
      historyIndex.value = -1
      commandInput.value = ''
    }
  }
}

function clearTerminal() {
  terminal?.clear()
}

watch(() => serverStore.activeServerId, () => {
  terminal?.clear()
  terminal?.writeln('\x1b[90mSwitching server...\x1b[0m')
  connectWebSocket()
})

onMounted(async () => {
  await nextTick()
  initTerminal()
  connectWebSocket()

  window.addEventListener('resize', () => {
    fitAddon?.fit()
  })
})

onUnmounted(() => {
  if (reconnectTimer) clearTimeout(reconnectTimer)
  if (ws) {
    ws.onclose = null
    ws.close()
  }
  terminal?.dispose()
})
</script>

<template>
  <div class="d-flex flex-column" style="height: calc(100vh - 100px);">
    <div class="d-flex align-center mb-3">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Console</div>
      <v-spacer />
      <v-btn
        v-for="qc in quickCommands"
        :key="qc.cmd"
        size="small"
        variant="tonal"
        color="primary"
        class="ml-2"
        @click="sendCommand(qc.cmd)"
      >
        {{ qc.label }}
      </v-btn>
      <v-btn
        size="small"
        variant="tonal"
        color="default"
        class="ml-2"
        prepend-icon="mdi-delete"
        @click="clearTerminal"
      >
        Clear
      </v-btn>
    </div>

    <v-card class="flex-grow-1 pa-0" style="overflow: hidden;">
      <div ref="terminalRef" style="height: 100%; width: 100%;" />
    </v-card>

    <v-card class="mt-3 pa-2">
      <v-text-field
        v-model="commandInput"
        placeholder="Type a command..."
        variant="plain"
        density="compact"
        hide-details
        single-line
        prepend-inner-icon="mdi-chevron-right"
        style="font-family: 'Cascadia Code', 'Fira Code', monospace;"
        @keydown="handleKeyDown"
        @keydown.enter="sendCommand()"
      />
    </v-card>
  </div>
</template>
