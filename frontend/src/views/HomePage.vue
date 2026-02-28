<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useServerStore } from '../stores/server'
import { serverApi } from '../services/api'
import ServerCard from '../components/ServerCard.vue'
import CreateServerDialog from '../components/CreateServerDialog.vue'
import type { GameServer } from '../types'

const router = useRouter()
const serverStore = useServerStore()
const createDialog = ref(false)
const deleteDialog = ref(false)
const deleteTarget = ref<GameServer | null>(null)

// Log panel state
const logPanelOpen = ref(false)
const logPanelExpanded = ref(true)
const logTarget = ref<GameServer | null>(null)
const logLines = ref<string[]>([])
const logStatus = ref('')
const logScrollEl = ref<HTMLElement | null>(null)
let logPollTimer: ReturnType<typeof setInterval> | null = null

let pollTimer: ReturnType<typeof setInterval> | null = null

// Auto-detect provisioning servers and open panel
const provisioningServers = computed(() =>
  serverStore.servers.filter(
    (s) => s.provisioningStatus !== 'ready'
  )
)

function openServer(server: GameServer) {
  router.push(`/servers/${server.id}`)
}

function confirmDelete(server: GameServer) {
  deleteTarget.value = server
  deleteDialog.value = true
}

async function executeDelete() {
  if (!deleteTarget.value) return
  await serverStore.deleteServer(deleteTarget.value.id)
  deleteDialog.value = false
  deleteTarget.value = null
}

function onServerCreated(id: string) {
  // Auto-open log panel for the new server
  const server = serverStore.servers.find((s) => s.id === id)
  if (server) {
    openLogs(server)
  } else {
    // Server might not be in store yet, try after next poll
    setTimeout(() => {
      const s = serverStore.servers.find((s) => s.id === id)
      if (s) openLogs(s)
    }, 2000)
  }
}

async function openLogs(server: GameServer) {
  logTarget.value = server
  logPanelOpen.value = true
  logPanelExpanded.value = true
  logLines.value = []
  logStatus.value = server.provisioningStatus
  await fetchProvisionLogs(server.id)
  startLogPolling()
}

function startLogPolling() {
  if (logPollTimer) clearInterval(logPollTimer)
  logPollTimer = setInterval(() => {
    if (logTarget.value) fetchProvisionLogs(logTarget.value.id)
  }, 3000)
}

function closeLogs() {
  logPanelOpen.value = false
  logTarget.value = null
  logLines.value = []
  if (logPollTimer) {
    clearInterval(logPollTimer)
    logPollTimer = null
  }
}

function toggleExpanded() {
  logPanelExpanded.value = !logPanelExpanded.value
}

async function fetchProvisionLogs(serverId: string) {
  try {
    const sApi = serverApi(serverId)
    const res = await sApi.get<{ status: string; log: string[] }>('/provision-status')
    logLines.value = res.data.log ?? []
    logStatus.value = res.data.status
    // Stop polling if done
    if (res.data.status === 'ready' || res.data.status === 'error') {
      if (logPollTimer) {
        clearInterval(logPollTimer)
        logPollTimer = null
      }
      serverStore.fetchServers()
    }
    // Auto-scroll
    await nextTick()
    if (logScrollEl.value) {
      logScrollEl.value.scrollTop = logScrollEl.value.scrollHeight
    }
  } catch {
    // silent
  }
}

async function deleteFromPanel() {
  if (!logTarget.value) return
  await serverStore.deleteServer(logTarget.value.id)
  closeLogs()
}

// Watch for new provisioning servers auto-open
watch(provisioningServers, (servers) => {
  if (servers.length > 0 && !logPanelOpen.value) {
    const active = servers.find(
      (s) => s.provisioningStatus !== 'error' && s.provisioningStatus !== 'ready'
    )
    if (active) openLogs(active)
  }
})

onMounted(async () => {
  await serverStore.fetchServers()
  pollTimer = setInterval(() => serverStore.fetchServers(), 10000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
  if (logPollTimer) clearInterval(logPollTimer)
})
</script>

<template>
  <div :style="{ paddingBottom: logPanelOpen ? (logPanelExpanded ? '340px' : '52px') : '0' }">
    <div class="d-flex align-center mb-6">
      <div class="text-h5 font-weight-medium" style="color: #e2e8f0;">Your Servers</div>
      <v-spacer />
      <v-btn color="primary" prepend-icon="mdi-plus" @click="createDialog = true">
        Create Server
      </v-btn>
    </div>

    <v-row v-if="serverStore.loading && serverStore.servers.length === 0">
      <v-col cols="12" class="text-center py-12">
        <v-progress-circular indeterminate color="primary" size="48" />
      </v-col>
    </v-row>

    <v-row v-else-if="serverStore.servers.length === 0">
      <v-col cols="12">
        <v-card class="pa-12 text-center">
          <v-icon size="64" color="medium-emphasis" class="mb-4">mdi-server-off</v-icon>
          <div class="text-h6 text-medium-emphasis mb-2">No servers yet</div>
          <div class="text-body-2 text-medium-emphasis mb-4">Create your first Rust server to get started.</div>
          <v-btn color="primary" prepend-icon="mdi-plus" @click="createDialog = true">Create Server</v-btn>
        </v-card>
      </v-col>
    </v-row>

    <v-row v-else>
      <v-col
        v-for="server in serverStore.servers"
        :key="server.id"
        cols="12"
        sm="6"
        md="4"
        lg="3"
      >
        <ServerCard
          :server="server"
          @click="openServer(server)"
          @delete="confirmDelete(server)"
          @logs="openLogs(server)"
        />
      </v-col>
    </v-row>

    <CreateServerDialog v-model="createDialog" @created="onServerCreated" />

    <!-- Delete confirmation dialog -->
    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Delete Server</v-card-title>
        <v-card-text>
          Are you sure you want to delete <strong>{{ deleteTarget?.name }}</strong>? This will remove the server configuration but will not delete the game files from disk.
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">Cancel</v-btn>
          <v-btn color="error" variant="flat" @click="executeDelete">Delete</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Collapsible footer log panel -->
    <Transition name="slide-up">
      <div v-if="logPanelOpen" class="log-panel">
        <!-- Header bar (always visible) -->
        <div class="log-panel-header" @click="toggleExpanded">
          <v-icon size="16" class="mr-2" style="color: #8b949e;">mdi-console</v-icon>
          <span class="log-panel-title">{{ logTarget?.name }}</span>
          <v-chip
            size="x-small"
            variant="tonal"
            class="ml-2"
            :color="logStatus === 'error' ? 'error' : logStatus === 'ready' ? 'success' : 'warning'"
          >
            {{ logStatus }}
          </v-chip>
          <v-progress-circular
            v-if="logStatus !== 'error' && logStatus !== 'ready'"
            indeterminate
            size="14"
            width="2"
            color="warning"
            class="ml-2"
          />
          <v-spacer />
          <v-btn
            v-if="logStatus === 'error' && logTarget?.source === 'dynamic'"
            size="x-small"
            variant="tonal"
            color="error"
            class="mr-2"
            @click.stop="deleteFromPanel"
          >
            Delete
          </v-btn>
          <v-btn
            icon
            size="x-small"
            variant="text"
            @click.stop="toggleExpanded"
          >
            <v-icon size="16">{{ logPanelExpanded ? 'mdi-chevron-down' : 'mdi-chevron-up' }}</v-icon>
          </v-btn>
          <v-btn
            icon
            size="x-small"
            variant="text"
            class="ml-1"
            @click.stop="closeLogs"
          >
            <v-icon size="16">mdi-close</v-icon>
          </v-btn>
        </div>

        <!-- Log content (collapsible) -->
        <div v-show="logPanelExpanded" ref="logScrollEl" class="log-panel-content">
          <div v-if="logLines.length === 0" class="text-medium-emphasis" style="font-size: 12px;">
            Waiting for output...
          </div>
          <div v-for="(line, i) in logLines" :key="i" class="log-line">
            <span class="log-line-num">{{ String(i + 1).padStart(3, ' ') }}</span>
            <span :class="lineClass(line)">{{ line }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script lang="ts">
function lineClass(line: string): string {
  if (line.includes('failed') || line.includes('Error') || line.includes('STDERR:') || line.includes('FAIL')) {
    return 'log-error'
  }
  if (line.includes('complete') || line.includes('installed') || line.includes('Oxide installed')) {
    return 'log-success'
  }
  return 'log-normal'
}
</script>

<style scoped>
.log-panel {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100;
  background: #0d1117;
  border-top: 1px solid #30363d;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.4);
}

.log-panel-header {
  display: flex;
  align-items: center;
  padding: 8px 16px;
  background: #161b22;
  border-bottom: 1px solid #21262d;
  cursor: pointer;
  user-select: none;
}

.log-panel-header:hover {
  background: #1c2128;
}

.log-panel-title {
  color: #e6edf3;
  font-size: 13px;
  font-weight: 500;
}

.log-panel-content {
  padding: 12px 16px;
  max-height: 280px;
  overflow-y: auto;
  font-family: 'Fira Code', 'Cascadia Code', 'SF Mono', monospace;
  font-size: 12px;
  line-height: 1.7;
}

.log-panel-content::-webkit-scrollbar {
  width: 6px;
}
.log-panel-content::-webkit-scrollbar-track {
  background: transparent;
}
.log-panel-content::-webkit-scrollbar-thumb {
  background: #30363d;
  border-radius: 3px;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
}

.log-line-num {
  color: #484f58;
  user-select: none;
  margin-right: 12px;
}

.log-error {
  color: #f85149;
}

.log-success {
  color: #3fb950;
}

.log-normal {
  color: #c9d1d9;
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: transform 0.25s ease, opacity 0.25s ease;
}
.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
