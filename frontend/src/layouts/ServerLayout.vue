<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useServerStore } from '../stores/server'
import { serverApi } from '../services/api'
import type { ServerInfo } from '../types'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const serverStore = useServerStore()
const drawer = ref(true)
const serverInfo = ref<ServerInfo | null>(null)
const confirmDialog = ref(false)
const confirmAction = ref<{ title: string; text: string; action: () => void } | null>(null)

const serverId = computed(() => route.params.serverId as string)

const navItems = computed(() => {
  const base = `/servers/${serverId.value}`
  return [
    { title: 'Dashboard', icon: 'mdi-view-dashboard', to: base },
    { title: 'Console', icon: 'mdi-console', to: `${base}/console` },
    { title: 'Players', icon: 'mdi-account-group', to: `${base}/players` },
    { title: 'Files', icon: 'mdi-folder', to: `${base}/files` },
    { title: 'Plugins', icon: 'mdi-puzzle', to: `${base}/plugins` },
    { title: 'Config', icon: 'mdi-cog', to: `${base}/config` },
    { title: 'Logs', icon: 'mdi-text-box-multiple', to: `${base}/logs` },
    { title: 'Schedule', icon: 'mdi-clock-outline', to: `${base}/schedule` },
    { title: 'Map', icon: 'mdi-map', to: `${base}/map` },
  ]
})

const serverName = computed(() => {
  const s = serverStore.getServer(serverId.value)
  return s?.name ?? serverId.value
})

let statusTimer: ReturnType<typeof setInterval> | null = null

async function fetchStatus() {
  if (!serverId.value) return
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<ServerInfo>('/status')
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
      const api = serverApi(serverId.value)
      await api.post('/restart')
    } catch { /* handled by interceptor */ }
  })
}

async function quickSave() {
  try {
    const api = serverApi(serverId.value)
    await api.post('/save')
  } catch { /* handled by interceptor */ }
}

function logout() {
  authStore.logout()
  router.push('/login')
}

onMounted(async () => {
  await serverStore.fetchServers()
  fetchStatus()
  statusTimer = setInterval(fetchStatus, 10000)
})

onUnmounted(() => {
  if (statusTimer) clearInterval(statusTimer)
})
</script>

<template>
  <v-navigation-drawer v-model="drawer" permanent width="240" class="bg-surface" style="border-right: 1px solid rgba(255,255,255,0.06);">
    <div class="pa-4 pb-2">
      <div class="d-flex align-center mb-1">
        <v-icon size="28" color="primary" class="mr-2">mdi-server</v-icon>
        <span class="text-h6 font-weight-medium" style="color: #e2e8f0; letter-spacing: -0.3px;">Panel</span>
      </div>
    </div>

    <!-- Back to Servers -->
    <div class="px-3 pb-2">
      <v-list density="compact" class="pa-0" nav>
        <v-list-item
          to="/"
          rounded="lg"
          density="compact"
          class="mb-1"
        >
          <template #prepend>
            <v-icon size="18" class="mr-2">mdi-arrow-left</v-icon>
          </template>
          <v-list-item-title class="text-body-2">Back to Servers</v-list-item-title>
        </v-list-item>
      </v-list>
    </div>

    <v-divider class="mx-3 my-1" style="opacity: 0.06;" />

    <!-- Server Name -->
    <div class="px-4 py-2">
      <div class="text-overline text-medium-emphasis" style="font-size: 10px;">SERVER</div>
      <div class="text-body-2 font-weight-medium" style="color: #e2e8f0;">{{ serverName }}</div>
    </div>

    <v-divider class="mx-3 my-1" style="opacity: 0.06;" />

    <!-- Navigation -->
    <div class="px-3 pt-1">
      <div class="text-overline text-medium-emphasis px-1 mb-1" style="font-size: 10px;">NAVIGATION</div>
      <v-list nav density="compact" class="pa-0">
        <v-list-item
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          :prepend-icon="item.icon"
          :title="item.title"
          rounded="lg"
          density="compact"
          class="mb-1"
          color="primary"
          exact
        />
      </v-list>
    </div>

    <template #append>
      <div class="pa-3">
        <v-btn
          block
          variant="text"
          color="medium-emphasis"
          prepend-icon="mdi-logout"
          size="small"
          @click="logout"
        >
          Logout
        </v-btn>
      </div>
    </template>
  </v-navigation-drawer>

  <v-app-bar flat class="bg-surface" border="b" density="compact" style="border-color: rgba(255,255,255,0.06) !important;">
    <v-app-bar-nav-icon size="small" @click="drawer = !drawer" />

    <v-chip
      :color="serverInfo?.online ? 'success' : 'error'"
      variant="tonal"
      size="small"
      class="ml-2"
    >
      <v-icon start size="8">mdi-circle</v-icon>
      {{ serverInfo?.online ? 'ONLINE' : 'OFFLINE' }}
    </v-chip>

    <v-chip
      v-if="serverInfo?.online"
      variant="text"
      size="small"
      class="ml-2 text-medium-emphasis"
    >
      <v-icon start size="14">mdi-account</v-icon>
      {{ serverInfo?.players ?? 0 }}/{{ serverInfo?.maxPlayers ?? 0 }}
    </v-chip>

    <v-spacer />

    <v-tooltip text="Restart Server" location="bottom">
      <template #activator="{ props }">
        <v-btn v-bind="props" icon="mdi-restart" size="small" variant="text" color="medium-emphasis" @click="quickRestart" />
      </template>
    </v-tooltip>
    <v-tooltip text="Save Server" location="bottom">
      <template #activator="{ props }">
        <v-btn v-bind="props" icon="mdi-content-save" size="small" variant="text" color="medium-emphasis" class="mr-2" @click="quickSave" />
      </template>
    </v-tooltip>
  </v-app-bar>

  <v-main class="bg-background">
    <v-container fluid class="pa-6">
      <router-view />
    </v-container>
  </v-main>

  <v-dialog v-model="confirmDialog" max-width="400">
    <v-card>
      <v-card-title class="text-h6 font-weight-medium">{{ confirmAction?.title }}</v-card-title>
      <v-card-text>{{ confirmAction?.text }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="confirmDialog = false">Cancel</v-btn>
        <v-btn color="primary" variant="flat" @click="executeConfirm">Confirm</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
