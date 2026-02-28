<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { serverApi } from '../services/api'
import { useRoute } from 'vue-router'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const logFile = ref('console')
const lineCount = ref(100)
const autoScroll = ref(true)
const autoRefresh = ref(true)
const loading = ref(true)
const logLines = ref<string[]>([])
const logOutputRef = ref<HTMLPreElement | null>(null)
let refreshTimer: ReturnType<typeof setInterval> | null = null

const logFiles = [
  { title: 'Console', value: 'console' },
  { title: 'Oxide', value: 'oxide' },
  { title: 'Script', value: 'script' },
]

const lineOptions = [
  { title: '100 lines', value: 100 },
  { title: '500 lines', value: 500 },
  { title: '1000 lines', value: 1000 },
]

async function fetchLogs() {
  if (!serverId.value) return
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ lines: string[] }>('/logs/tail', {
      params: { file: logFile.value, lines: lineCount.value },
    })
    logLines.value = res.data.lines ?? []
    if (autoScroll.value) {
      await nextTick()
      scrollToBottom()
    }
  } catch {
    logLines.value = ['Failed to load logs.']
  } finally {
    loading.value = false
  }
}

function scrollToBottom() {
  if (logOutputRef.value) {
    logOutputRef.value.scrollTop = logOutputRef.value.scrollHeight
  }
}

function startAutoRefresh() {
  stopAutoRefresh()
  if (autoRefresh.value) {
    refreshTimer = setInterval(fetchLogs, 3000)
  }
}

function stopAutoRefresh() {
  if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null }
}

function clearLog() { logLines.value = [] }

watch([logFile, lineCount], () => { loading.value = true; fetchLogs() })
watch(autoRefresh, (val) => { if (val) startAutoRefresh(); else stopAutoRefresh() })

onMounted(() => { fetchLogs(); startAutoRefresh() })
onUnmounted(() => { stopAutoRefresh() })
</script>

<template>
  <div class="d-flex flex-column" style="height: calc(100vh - 100px);">
    <div class="d-flex align-center mb-3 flex-wrap ga-2">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Log Viewer</div>
      <v-spacer />
      <v-select v-model="logFile" :items="logFiles" density="compact" variant="outlined" hide-details style="max-width: 150px;" />
      <v-select v-model="lineCount" :items="lineOptions" density="compact" variant="outlined" hide-details style="max-width: 150px;" />
      <v-btn-toggle v-model="autoScroll" density="compact" variant="outlined" color="primary">
        <v-btn :value="true" size="small"><v-icon start>mdi-arrow-down-bold</v-icon>Auto-scroll</v-btn>
      </v-btn-toggle>
      <v-btn-toggle v-model="autoRefresh" density="compact" variant="outlined" color="primary">
        <v-btn :value="true" size="small"><v-icon start>mdi-refresh-auto</v-icon>Live</v-btn>
      </v-btn-toggle>
      <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="fetchLogs">Refresh</v-btn>
      <v-btn size="small" variant="tonal" color="default" prepend-icon="mdi-delete" @click="clearLog">Clear</v-btn>
    </div>

    <v-card class="flex-grow-1 pa-0" style="overflow: hidden;">
      <div v-if="loading" class="d-flex justify-center align-center fill-height pa-8">
        <v-progress-circular indeterminate color="primary" size="36" />
      </div>
      <pre
        v-else
        ref="logOutputRef"
        style="height: 100%; margin: 0; overflow: auto; background: #0a0a0b; color: #33ff33; padding: 12px; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 12px; line-height: 1.4; white-space: pre-wrap; word-break: break-all;"
      >{{ logLines.join('\n') }}</pre>
    </v-card>
  </div>
</template>
