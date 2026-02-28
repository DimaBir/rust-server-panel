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
import { serverApi } from '../services/api'
import api from '../services/api'
import { useRoute } from 'vue-router'
import type { ServerInfo, SystemStats, GameStats, MonitorResponse } from '../types'

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, Filler)

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const serverInfo = ref<ServerInfo | null>(null)
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
  if (!serverId.value) return
  const sApi = serverApi(serverId.value)
  try {
    const [statusRes, systemRes, gameRes] = await Promise.allSettled([
      sApi.get<ServerInfo>('/status'),
      api.get<MonitorResponse<SystemStats>>('/monitor/system'),
      sApi.get<MonitorResponse<GameStats>>('/monitor/game'),
    ])

    if (statusRes.status === 'fulfilled') {
      serverInfo.value = statusRes.value.data
    }
    if (systemRes.status === 'fulfilled') {
      const current = systemRes.value.data.current
      if (current) {
        cpuHistory.value = [...cpuHistory.value, current.cpuPercent].slice(-MAX_HISTORY)
        memHistory.value = [...memHistory.value, current.memPercent].slice(-MAX_HISTORY)
      }
    }
    if (gameRes.status === 'fulfilled') {
      const current = gameRes.value.data.current
      if (current) {
        playerHistory.value = [...playerHistory.value, current.players].slice(-MAX_HISTORY)
      }
    }

    const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
    timeLabels.value = [...timeLabels.value, now].slice(-MAX_HISTORY)
  } catch {
    // Silent poll
  } finally {
    loading.value = false
  }
}

function showConfirm(title: string, text: string, action: () => void) {
  confirmAction.value = { title, text, action }
  confirmDialog.value = true
}

async function executeConfirm() {
  if (confirmAction.value) confirmAction.value.action()
  confirmDialog.value = false
}

const quickActions = [
  { label: 'Start', icon: 'mdi-play', endpoint: '/start' },
  { label: 'Stop', icon: 'mdi-stop', endpoint: '/stop' },
  { label: 'Restart', icon: 'mdi-restart', endpoint: '/restart' },
  { label: 'Update', icon: 'mdi-download', endpoint: '/update' },
  { label: 'Save', icon: 'mdi-content-save', endpoint: '/save' },
  { label: 'Backup', icon: 'mdi-backup-restore', endpoint: '/backup' },
]

function handleQuickAction(action: typeof quickActions[0]) {
  showConfirm(
    action.label,
    `Are you sure you want to ${action.label.toLowerCase()} the server?`,
    async () => {
      try {
        const sApi = serverApi(serverId.value)
        await sApi.post(action.endpoint)
      } catch { /* interceptor */ }
    }
  )
}

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h}h ${m}m`
}

function formatBytes(bytes: number | undefined): string {
  if (bytes == null || isNaN(bytes)) return '—'
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
      borderColor: '#3b82f6',
      backgroundColor: 'rgba(59, 130, 246, 0.08)',
      fill: true,
      tension: 0.4,
      borderWidth: 2,
      pointRadius: 2,
    },
    {
      label: 'Memory %',
      data: memHistory.value,
      borderColor: '#10b981',
      backgroundColor: 'rgba(16, 185, 129, 0.08)',
      fill: true,
      tension: 0.4,
      borderWidth: 2,
      pointRadius: 2,
    },
  ],
}))

const playerChartData = computed(() => ({
  labels: timeLabels.value,
  datasets: [
    {
      label: 'Players',
      data: playerHistory.value,
      borderColor: '#f59e0b',
      backgroundColor: 'rgba(245, 158, 11, 0.08)',
      fill: true,
      tension: 0.4,
      borderWidth: 2,
      pointRadius: 2,
    },
  ],
}))

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      labels: { color: '#94a3b8', font: { size: 11 } },
    },
  },
  scales: {
    x: {
      ticks: { color: '#475569', maxTicksLimit: 6, font: { size: 10 } },
      grid: { color: 'rgba(255,255,255,0.03)' },
    },
    y: {
      ticks: { color: '#475569', font: { size: 10 } },
      grid: { color: 'rgba(255,255,255,0.03)' },
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
    <div class="text-h6 font-weight-medium mb-5" style="color: #e2e8f0;">Dashboard</div>

    <v-row v-if="loading">
      <v-col cols="12" class="text-center py-12">
        <v-progress-circular indeterminate color="primary" size="48" />
      </v-col>
    </v-row>

    <template v-else>
      <!-- Stat Cards -->
      <v-row class="mb-2">
        <v-col cols="6" md="3">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-1">Players</div>
            <div class="text-h4 font-weight-medium" style="color: #e2e8f0;">
              {{ serverInfo?.online ? serverInfo.players : '—' }}
            </div>
            <div class="text-caption text-medium-emphasis" v-if="serverInfo?.online">
              of {{ serverInfo.maxPlayers }}
            </div>
            <div class="text-caption text-medium-emphasis" v-else>Offline</div>
          </v-card>
        </v-col>
        <v-col cols="6" md="3">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-1">CPU</div>
            <div class="text-h4 font-weight-medium" style="color: #e2e8f0;">
              {{ serverInfo?.cpuPercent != null ? serverInfo.cpuPercent.toFixed(0) + '%' : '—' }}
            </div>
            <div class="text-caption text-medium-emphasis">Usage</div>
          </v-card>
        </v-col>
        <v-col cols="6" md="3">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-1">Memory</div>
            <div class="text-h4 font-weight-medium" style="color: #e2e8f0;">
              {{ serverInfo?.memPercent != null ? serverInfo.memPercent.toFixed(0) + '%' : '—' }}
            </div>
            <div class="text-caption text-medium-emphasis">
              {{ formatBytes(serverInfo?.memUsed) }} / {{ formatBytes(serverInfo?.memTotal) }}
            </div>
          </v-card>
        </v-col>
        <v-col cols="6" md="3">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-1">FPS</div>
            <div class="text-h4 font-weight-medium" style="color: #e2e8f0;">
              {{ serverInfo?.online ? serverInfo.fps?.toFixed(0) ?? '—' : '—' }}
            </div>
            <div class="text-caption text-medium-emphasis" v-if="serverInfo?.online">
              {{ serverInfo.entities?.toLocaleString() ?? 0 }} entities
            </div>
            <div class="text-caption text-medium-emphasis" v-else>Offline</div>
          </v-card>
        </v-col>
      </v-row>

      <!-- Server Info Strip + Quick Actions -->
      <v-row>
        <v-col cols="12" md="8">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-3">Server Info</div>
            <v-row dense>
              <v-col cols="6" sm="4">
                <div class="text-caption text-medium-emphasis">Hostname</div>
                <div class="text-body-2" style="color: #e2e8f0;">{{ serverInfo?.hostname || '—' }}</div>
              </v-col>
              <v-col cols="6" sm="4">
                <div class="text-caption text-medium-emphasis">Map</div>
                <div class="text-body-2" style="color: #e2e8f0;">{{ serverInfo?.map || '—' }}</div>
              </v-col>
              <v-col cols="6" sm="4">
                <div class="text-caption text-medium-emphasis">Uptime</div>
                <div class="text-body-2" style="color: #e2e8f0;">{{ serverInfo?.uptime ? formatUptime(serverInfo.uptime) : '—' }}</div>
              </v-col>
            </v-row>
          </v-card>
        </v-col>

        <v-col cols="12" md="4">
          <v-card class="pa-4 fill-height">
            <div class="text-caption text-medium-emphasis mb-3">Quick Actions</div>
            <div class="d-flex flex-wrap ga-2">
              <v-tooltip v-for="action in quickActions" :key="action.label" :text="action.label" location="top">
                <template #activator="{ props }">
                  <v-btn
                    v-bind="props"
                    :icon="action.icon"
                    size="small"
                    variant="tonal"
                    color="primary"
                    @click="handleQuickAction(action)"
                  />
                </template>
              </v-tooltip>
            </div>
          </v-card>
        </v-col>
      </v-row>

      <!-- Charts -->
      <v-row class="mt-2">
        <v-col cols="12" md="6">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-3">System Performance</div>
            <div style="height: 220px;">
              <Line :data="systemChartData" :options="chartOptions" />
            </div>
          </v-card>
        </v-col>
        <v-col cols="12" md="6">
          <v-card class="pa-4">
            <div class="text-caption text-medium-emphasis mb-3">Player Count</div>
            <div style="height: 220px;">
              <Line :data="playerChartData" :options="chartOptions" />
            </div>
          </v-card>
        </v-col>
      </v-row>
    </template>

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
  </div>
</template>
