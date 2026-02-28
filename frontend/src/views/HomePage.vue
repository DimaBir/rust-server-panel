<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useServerStore } from '../stores/server'
import ServerCard from '../components/ServerCard.vue'
import CreateServerDialog from '../components/CreateServerDialog.vue'
import type { GameServer } from '../types'

const router = useRouter()
const serverStore = useServerStore()
const createDialog = ref(false)
const deleteDialog = ref(false)
const deleteTarget = ref<GameServer | null>(null)

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

onMounted(async () => {
  await serverStore.fetchServers()
  pollTimer = setInterval(() => serverStore.fetchServers(), 10000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
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
        />
      </v-col>
    </v-row>

    <CreateServerDialog v-model="createDialog" @created="onServerCreated" />

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
  </div>
</template>
