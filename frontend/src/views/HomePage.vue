<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
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

// Provisioning log dialog
const logDialog = ref(false)
const logTarget = ref<GameServer | null>(null)
const logLines = ref<string[]>([])
const logStatus = ref('')
const logLoading = ref(false)
let logPollTimer: ReturnType<typeof setInterval> | null = null

let pollTimer: ReturnType<typeof setInterval> | null = null

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

function onServerCreated(_id: string) {
  // Server list will auto-refresh via polling
}

async function openLogs(server: GameServer) {
  logTarget.value = server
  logDialog.value = true
  logLines.value = []
  logStatus.value = server.provisioningStatus
  await fetchProvisionLogs(server.id)
  // Poll while still provisioning
  if (logPollTimer) clearInterval(logPollTimer)
  logPollTimer = setInterval(() => {
    if (logTarget.value) fetchProvisionLogs(logTarget.value.id)
  }, 3000)
}

function closeLogs() {
  logDialog.value = false
  logTarget.value = null
  if (logPollTimer) {
    clearInterval(logPollTimer)
    logPollTimer = null
  }
}

async function fetchProvisionLogs(serverId: string) {
  try {
    logLoading.value = true
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
      // Refresh server list
      serverStore.fetchServers()
    }
  } catch {
    // silent
  } finally {
    logLoading.value = false
  }
}

async function deleteFromLogs() {
  if (!logTarget.value) return
  await serverStore.deleteServer(logTarget.value.id)
  closeLogs()
}

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
  <div>
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

    <!-- Provisioning log dialog -->
    <v-dialog v-model="logDialog" max-width="700" @after-leave="closeLogs">
      <v-card>
        <v-card-title class="d-flex align-center">
          <v-icon class="mr-2" size="20">mdi-console</v-icon>
          <span class="text-h6 font-weight-medium">{{ logTarget?.name }} — Provisioning</span>
          <v-spacer />
          <v-chip
            size="small"
            variant="tonal"
            :color="logStatus === 'error' ? 'error' : logStatus === 'ready' ? 'success' : 'warning'"
          >
            {{ logStatus }}
          </v-chip>
        </v-card-title>

        <v-card-text class="pa-0">
          <div
            class="provision-log pa-4"
            style="background: #0d1117; max-height: 400px; overflow-y: auto; font-family: 'Fira Code', 'Cascadia Code', monospace; font-size: 12px; line-height: 1.6;"
          >
            <div v-if="logLines.length === 0 && logLoading" class="text-medium-emphasis">
              Loading logs...
            </div>
            <div v-else-if="logLines.length === 0" class="text-medium-emphasis">
              No log output yet.
            </div>
            <div v-for="(line, i) in logLines" :key="i" style="white-space: pre-wrap; word-break: break-all;">
              <span style="color: #484f58; user-select: none;">{{ String(i + 1).padStart(3, ' ') }}  </span>
              <span :style="{ color: line.includes('failed') || line.includes('Error') || line.includes('STDERR') ? '#f85149' : line.includes('complete') || line.includes('installed') ? '#3fb950' : '#c9d1d9' }">{{ line }}</span>
            </div>

            <!-- Spinner for active provisioning -->
            <div v-if="logStatus !== 'error' && logStatus !== 'ready'" class="mt-2 d-flex align-center">
              <v-progress-circular indeterminate size="14" width="2" color="warning" class="mr-2" />
              <span style="color: #f59e0b; font-size: 12px;">Provisioning in progress...</span>
            </div>
          </div>
        </v-card-text>

        <v-card-actions>
          <v-btn
            v-if="logStatus === 'error' && logTarget?.source === 'dynamic'"
            color="error"
            variant="tonal"
            size="small"
            prepend-icon="mdi-delete"
            @click="deleteFromLogs"
          >
            Delete Server
          </v-btn>
          <v-spacer />
          <v-btn variant="text" @click="closeLogs">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.provision-log::-webkit-scrollbar {
  width: 6px;
}
.provision-log::-webkit-scrollbar-track {
  background: transparent;
}
.provision-log::-webkit-scrollbar-thumb {
  background: #30363d;
  border-radius: 3px;
}
</style>
