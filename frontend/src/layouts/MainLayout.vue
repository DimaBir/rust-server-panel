<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import api from '../services/api'
import type { ServerInfo } from '../types'

const router = useRouter()
const authStore = useAuthStore()
const drawer = ref(true)
const serverInfo = ref<ServerInfo | null>(null)
const confirmDialog = ref(false)
const confirmAction = ref<{ title: string; text: string; action: () => void } | null>(null)

const navItems = [
  { title: 'Dashboard', icon: 'mdi-view-dashboard', to: '/' },
  { title: 'Console', icon: 'mdi-console', to: '/console' },
  { title: 'Players', icon: 'mdi-account-group', to: '/players' },
  { title: 'Files', icon: 'mdi-folder', to: '/files' },
  { title: 'Plugins', icon: 'mdi-puzzle', to: '/plugins' },
  { title: 'Config', icon: 'mdi-cog', to: '/config' },
  { title: 'Logs', icon: 'mdi-text-box-multiple', to: '/logs' },
  { title: 'Schedule', icon: 'mdi-clock-outline', to: '/schedule' },
]

let statusTimer: ReturnType<typeof setInterval> | null = null

async function fetchStatus() {
  try {
    const res = await api.get<ServerInfo>('/server/status')
    serverInfo.value = res.data
  } catch {
    serverInfo.value = null
  }
}

function showConfirm(title: string, text: string, action: () => void) {
  confirmAction.value = { title, text, action }
  confirmDialog.value = true
}

async function executeConfirm() {
  if (confirmAction.value) {
    confirmAction.value.action()
  }
  confirmDialog.value = false
}

async function quickRestart() {
  showConfirm('Restart Server', 'Are you sure you want to restart the server?', async () => {
    try {
      await api.post('/server/restart')
    } catch { /* handled by interceptor */ }
  })
}

async function quickSave() {
  try {
    await api.post('/server/save')
  } catch { /* handled by interceptor */ }
}

function logout() {
  authStore.logout()
  router.push('/login')
}

onMounted(() => {
  fetchStatus()
  statusTimer = setInterval(fetchStatus, 10000)
})

onUnmounted(() => {
  if (statusTimer) clearInterval(statusTimer)
})
</script>

<template>
  <v-navigation-drawer v-model="drawer" permanent class="bg-surface">
    <div class="pa-4 text-center">
      <v-icon size="40" color="primary" class="mb-2">mdi-server</v-icon>
      <div class="text-h6 font-weight-bold" style="color: #CD412B;">RUST PANEL</div>
      <div class="text-caption text-grey">Server Control</div>
    </div>

    <v-divider class="mb-2" />

    <v-list nav density="comfortable">
      <v-list-item
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        :prepend-icon="item.icon"
        :title="item.title"
        rounded="lg"
        class="mb-1 mx-2"
        color="primary"
      />
    </v-list>

    <template #append>
      <div class="pa-3">
        <v-btn
          block
          variant="outlined"
          color="grey"
          prepend-icon="mdi-logout"
          @click="logout"
        >
          Logout
        </v-btn>
      </div>
    </template>
  </v-navigation-drawer>

  <v-app-bar flat class="bg-surface" border="b">
    <v-app-bar-nav-icon @click="drawer = !drawer" />

    <v-chip
      :color="serverInfo?.online ? 'success' : 'error'"
      variant="flat"
      size="small"
      class="ml-2"
    >
      <v-icon start size="10">mdi-circle</v-icon>
      {{ serverInfo?.online ? 'ONLINE' : 'OFFLINE' }}
    </v-chip>

    <v-chip
      v-if="serverInfo?.online"
      variant="tonal"
      size="small"
      class="ml-2"
      color="info"
    >
      <v-icon start size="14">mdi-account</v-icon>
      {{ serverInfo?.players ?? 0 }} / {{ serverInfo?.maxPlayers ?? 0 }}
    </v-chip>

    <v-spacer />

    <v-btn
      icon="mdi-restart"
      size="small"
      variant="text"
      color="warning"
      title="Restart Server"
      @click="quickRestart"
    />
    <v-btn
      icon="mdi-content-save"
      size="small"
      variant="text"
      color="success"
      title="Save Server"
      class="mr-2"
      @click="quickSave"
    />
  </v-app-bar>

  <v-main class="bg-background">
    <v-container fluid class="pa-6">
      <router-view />
    </v-container>
  </v-main>

  <v-dialog v-model="confirmDialog" max-width="400">
    <v-card>
      <v-card-title>{{ confirmAction?.title }}</v-card-title>
      <v-card-text>{{ confirmAction?.text }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="confirmDialog = false">Cancel</v-btn>
        <v-btn color="primary" variant="flat" @click="executeConfirm">Confirm</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
