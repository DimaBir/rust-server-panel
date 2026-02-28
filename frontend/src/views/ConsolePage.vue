<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import { useRoute } from 'vue-router'
import { serverApi } from '../services/api'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const terminalRef = ref<HTMLElement | null>(null)
const commandInput = ref('')
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

const quickCommands = [
  { label: 'status', cmd: 'status' },
  { label: 'serverinfo', cmd: 'serverinfo' },
  { label: 'players', cmd: 'global.playerlist' },
  { label: 'save', cmd: 'server.save' },
]

const serverCommands = [
  { label: 'Start', endpoint: '/start' },
  { label: 'Stop', endpoint: '/stop' },
  { label: 'Restart', endpoint: '/restart' },
  { label: 'Update', endpoint: '/update' },
  { label: 'Force Update', endpoint: '/force-update' },
  { label: 'Validate', endpoint: '/validate' },
  { label: 'Check Update', endpoint: '/check-update' },
  { label: 'Backup', endpoint: '/backup' },
  { label: 'Monitor', endpoint: '/monitor-check' },
  { label: 'Details', endpoint: '/details' },
  { label: 'Update LGSM', endpoint: '/update-lgsm' },
  { label: 'Map Wipe', endpoint: '/map-wipe' },
  { label: 'Full Wipe', endpoint: '/full-wipe' },
]

const runningCommand = ref<string | null>(null)

async function runServerCommand(cmd: typeof serverCommands[0]) {
  if (runningCommand.value) return
  runningCommand.value = cmd.endpoint
  terminal?.writeln(`\x1b[1;36m[LGSM] ${cmd.label}...\x1b[0m`)
  try {
    const sApi = serverApi(serverId.value)
    const res = await sApi.post<{ success: boolean; output: string }>(cmd.endpoint)
    const color = res.data.success ? '32' : '31'
    for (const line of res.data.output.split('\n')) {
      terminal?.writeln(`\x1b[${color}m${line}\x1b[0m`)
    }
  } catch (e: any) {
    terminal?.writeln(`\x1b[31m[ERROR] ${e.message || 'Command failed'}\x1b[0m`)
  } finally {
    runningCommand.value = null
  }
}

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
  if (!serverId.value) return
  if (ws) {
    ws.onclose = null
    ws.close()
  }
  if (reconnectTimer) clearTimeout(reconnectTimer)

  const token = localStorage.getItem('jwt_token')
  const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const url = `${wsProtocol}//${window.location.host}/ws/${serverId.value}/console${token ? '?token=' + token : ''}`

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

    <div class="d-flex align-center flex-wrap ga-1 mb-2">
      <span class="text-caption text-medium-emphasis mr-2">Server:</span>
      <v-btn
        v-for="cmd in serverCommands"
        :key="cmd.endpoint"
        size="x-small"
        variant="tonal"
        :color="cmd.endpoint.includes('wipe') ? 'error' : 'secondary'"
        :loading="runningCommand === cmd.endpoint"
        :disabled="runningCommand !== null"
        @click="runServerCommand(cmd)"
      >
        {{ cmd.label }}
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
