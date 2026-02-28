<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from 'chart.js'
import api from '../services/api'
import type { ServerInfo, SystemStats, GameStats } from '../types'

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, Filler)

const serverInfo = ref<ServerInfo | null>(null)
const systemStats = ref<SystemStats | null>(null)
const loading = ref(true)
const confirmDialog = ref(false)
const confirmAction = ref<{ title: string; text: string; action: () => void } | null>(null)

const cpuHistory = ref<number[]>([])
const memHistory = ref<number[]>([])
const playerHistory = ref<number[]>([])
const timeLabels = ref<string[]>([])

const MAX_HISTORY = 30

let pollTimer: ReturnType<typeof setInterval> | null = null

async function fetchData() {
  try {
    const [statusRes, systemRes, gameRes] = await Promise.allSettled([
      api.get<ServerInfo>('/server/status'),
      api.get<SystemStats>('/monitor/system'),
      api.get<GameStats>('/monitor/game'),
    ])

    if (statusRes.status === 'fulfilled') {
      serverInfo.value = statusRes.value.data
    }
    if (systemRes.status === 'fulfilled') {
      systemStats.value = systemRes.value.data
      cpuHistory.value.push(systemRes.value.data.cpuPercent)
      memHistory.value.push(systemRes.value.data.memPercent)
      if (cpuHistory.value.length > MAX_HISTORY) cpuHistory.value.shift()
      if (memHistory.value.length > MAX_HISTORY) memHistory.value.shift()
    }
    if (gameRes.status === 'fulfilled') {
      playerHistory.value.push(gameRes.value.data.players)
      if (playerHistory.value.length > MAX_HISTORY) playerHistory.value.shift()
    }

    const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
    timeLabels.value.push(now)
    if (timeLabels.value.length > MAX_HISTORY) timeLabels.value.shift()
  } catch {
    // Errors handled silently for dashboard polling
  } finally {
    loading.value = false
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

const quickActions = [
  { label: 'Start', icon: 'mdi-play', color: 'success', endpoint: '/server/start' },
  { label: 'Stop', icon: 'mdi-stop', color: 'error', endpoint: '/server/stop' },
  { label: 'Restart', icon: 'mdi-restart', color: 'warning', endpoint: '/server/restart' },
  { label: 'Update', icon: 'mdi-download', color: 'info', endpoint: '/server/update' },
  { label: 'Save', icon: 'mdi-content-save', color: 'success', endpoint: '/server/save' },
  { label: 'Wipe Map', icon: 'mdi-map-marker-off', color: 'warning', endpoint: '/server/wipe/map' },
  { label: 'Full Wipe', icon: 'mdi-delete-forever', color: 'error', endpoint: '/server/wipe/full' },
]

function handleQuickAction(action: typeof quickActions[0]) {
  showConfirm(
    action.label,
    `Are you sure you want to ${action.label.toLowerCase()} the server?`,
    async () => {
      try { await api.post(action.endpoint) } catch { /* interceptor handles */ }
    }
  )
}

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h}h ${m}m`
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  const gb = bytes / (1024 * 1024 * 1024)
  if (gb >= 1) return gb.toFixed(1) + ' GB'
  const mb = bytes / (1024 * 1024)
  return mb.toFixed(0) + ' MB'
}

const systemChartData = computed(() => ({
  labels: timeLabels.value,
  datasets: [
    {
      label: 'CPU %',
      data: cpuHistory.value,
      borderColor: '#CD412B',
      backgroundColor: 'rgba(205, 65, 43, 0.1)',
      fill: true,
      tension: 0.3,
    },
    {
      label: 'Memory %',
      data: memHistory.value,
      borderColor: '#4FC3F7',
      backgroundColor: 'rgba(79, 195, 247, 0.1)',
      fill: true,
      tension: 0.3,
    },
  ],
}))

const playerChartData = computed(() => ({
  labels: timeLabels.value,
  datasets: [
    {
      label: 'Players',
      data: playerHistory.value,
      borderColor: '#66BB6A',
      backgroundColor: 'rgba(102, 187, 106, 0.1)',
      fill: true,
      tension: 0.3,
    },
  ],
}))

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      labels: { color: '#E0E0E0' },
    },
  },
  scales: {
    x: {
      ticks: { color: '#999', maxTicksLimit: 8 },
      grid: { color: 'rgba(255,255,255,0.05)' },
    },
    y: {
      ticks: { color: '#999' },
      grid: { color: 'rgba(255,255,255,0.05)' },
      min: 0,
    },
  },
}

onMounted(() => {
  fetchData()
  pollTimer = setInterval(fetchData, 5000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div>
    <div class="text-h5 font-weight-bold mb-4">Dashboard</div>

    <v-row v-if="loading">
      <v-col cols="12" class="text-center py-12">
        <v-progress-circular indeterminate color="primary" size="48" />
      </v-col>
    </v-row>

    <template v-else>
      <!-- Server Status -->
      <v-row>
        <v-col cols="12" md="8">
          <v-card>
            <v-card-title class="d-flex align-center">
              <v-icon class="mr-2">mdi-server</v-icon>
              Server Status
              <v-spacer />
              <v-chip
                :color="serverInfo?.online ? 'success' : 'error'"
                size="small"
                variant="flat"
              >
                {{ serverInfo?.online ? 'ONLINE' : 'OFFLINE' }}
              </v-chip>
            </v-card-title>
            <v-card-text>
              <v-row dense>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">Hostname</div>
                  <div class="text-body-2">{{ serverInfo?.hostname ?? 'N/A' }}</div>
                </v-col>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">Map</div>
                  <div class="text-body-2">{{ serverInfo?.map ?? 'N/A' }}</div>
                </v-col>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">Players</div>
                  <div class="text-body-2">{{ serverInfo?.players ?? 0 }} / {{ serverInfo?.maxPlayers ?? 0 }}</div>
                </v-col>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">FPS</div>
                  <div class="text-body-2">{{ serverInfo?.fps ?? 0 }}</div>
                </v-col>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">Entities</div>
                  <div class="text-body-2">{{ serverInfo?.entities ?? 0 }}</div>
                </v-col>
                <v-col cols="6" sm="4">
                  <div class="text-caption text-grey">Uptime</div>
                  <div class="text-body-2">{{ serverInfo?.uptime ? formatUptime(serverInfo.uptime) : 'N/A' }}</div>
                </v-col>
              </v-row>
            </v-card-text>
          </v-card>
        </v-col>

        <v-col cols="12" md="4">
          <v-card class="fill-height">
            <v-card-title>
              <v-icon class="mr-2">mdi-lightning-bolt</v-icon>
              Quick Actions
            </v-card-title>
            <v-card-text>
              <div class="d-flex flex-wrap ga-2">
                <v-btn
                  v-for="action in quickActions"
                  :key="action.label"
                  :color="action.color"
                  :prepend-icon="action.icon"
                  size="small"
                  variant="tonal"
                  @click="handleQuickAction(action)"
                >
                  {{ action.label }}
                </v-btn>
              </div>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>

      <!-- System Stats -->
      <v-row class="mt-2">
        <v-col cols="12" sm="4">
          <v-card class="text-center pa-4">
            <v-progress-circular
              :model-value="systemStats?.cpuPercent ?? 0"
              :size="80"
              :width="8"
              color="primary"
            >
              {{ (systemStats?.cpuPercent ?? 0).toFixed(0) }}%
            </v-progress-circular>
            <div class="text-body-1 mt-2">CPU Usage</div>
          </v-card>
        </v-col>
        <v-col cols="12" sm="4">
          <v-card class="text-center pa-4">
            <v-progress-circular
              :model-value="systemStats?.memPercent ?? 0"
              :size="80"
              :width="8"
              color="info"
            >
              {{ (systemStats?.memPercent ?? 0).toFixed(0) }}%
            </v-progress-circular>
            <div class="text-body-1 mt-2">Memory</div>
            <div class="text-caption text-grey">
              {{ systemStats ? formatBytes(systemStats.memUsed) : '0' }} / {{ systemStats ? formatBytes(systemStats.memTotal) : '0' }}
            </div>
          </v-card>
        </v-col>
        <v-col cols="12" sm="4">
          <v-card class="text-center pa-4">
            <v-progress-circular
              :model-value="systemStats?.diskPercent ?? 0"
              :size="80"
              :width="8"
              color="warning"
            >
              {{ (systemStats?.diskPercent ?? 0).toFixed(0) }}%
            </v-progress-circular>
            <div class="text-body-1 mt-2">Disk</div>
            <div class="text-caption text-grey">
              {{ systemStats ? formatBytes(systemStats.diskUsed) : '0' }} / {{ systemStats ? formatBytes(systemStats.diskTotal) : '0' }}
            </div>
          </v-card>
        </v-col>
      </v-row>

      <!-- Charts -->
      <v-row class="mt-2">
        <v-col cols="12" md="6">
          <v-card>
            <v-card-title>
              <v-icon class="mr-2">mdi-chart-line</v-icon>
              System Performance
            </v-card-title>
            <v-card-text>
              <div style="height: 250px;">
                <Line :data="systemChartData" :options="chartOptions" />
              </div>
            </v-card-text>
          </v-card>
        </v-col>
        <v-col cols="12" md="6">
          <v-card>
            <v-card-title>
              <v-icon class="mr-2">mdi-account-multiple</v-icon>
              Player Count
            </v-card-title>
            <v-card-text>
              <div style="height: 250px;">
                <Line :data="playerChartData" :options="chartOptions" />
              </div>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </template>

    <!-- Confirm Dialog -->
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
  </div>
</template>
