<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import api from '../services/api'

const logFile = ref('console')
const lineCount = ref(100)
const autoScroll = ref(true)
const autoRefresh = ref(true)
const loading = ref(true)
const logContent = ref('')

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
  try {
    const res = await api.get<{ content: string }>('/logs/tail', {
      params: { file: logFile.value, lines: lineCount.value },
    })
    logContent.value = res.data.content ?? ''
    if (autoScroll.value) {
      await nextTick()
      scrollToBottom()
    }
  } catch {
    logContent.value = 'Failed to load logs.'
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
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

function clearLog() {
  logContent.value = ''
}

watch([logFile, lineCount], () => {
  loading.value = true
  fetchLogs()
})

watch(autoRefresh, (val) => {
  if (val) {
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
})

onMounted(() => {
  fetchLogs()
  startAutoRefresh()
})

onUnmounted(() => {
  stopAutoRefresh()
})
</script>

<template>
  <div class="d-flex flex-column" style="height: calc(100vh - 100px);">
    <div class="d-flex align-center mb-3 flex-wrap ga-2">
      <div class="text-h5 font-weight-bold">Log Viewer</div>
      <v-spacer />

      <v-select
        v-model="logFile"
        :items="logFiles"
        density="compact"
        variant="outlined"
        hide-details
        style="max-width: 160px;"
      />

      <v-select
        v-model="lineCount"
        :items="lineOptions"
        density="compact"
        variant="outlined"
        hide-details
        style="max-width: 160px;"
      />

      <v-btn-toggle
        v-model="autoScroll"
        density="compact"
        variant="outlined"
        color="primary"
      >
        <v-btn :value="true" size="small">
          <v-icon start>mdi-arrow-down-bold</v-icon>
          Auto-scroll
        </v-btn>
      </v-btn-toggle>

      <v-btn-toggle
        v-model="autoRefresh"
        density="compact"
        variant="outlined"
        color="primary"
      >
        <v-btn :value="true" size="small">
          <v-icon start>mdi-refresh-auto</v-icon>
          Live
        </v-btn>
      </v-btn-toggle>

      <v-btn
        size="small"
        variant="tonal"
        color="info"
        prepend-icon="mdi-refresh"
        @click="fetchLogs"
      >
        Refresh
      </v-btn>

      <v-btn
        size="small"
        variant="tonal"
        color="grey"
        prepend-icon="mdi-delete"
        @click="clearLog"
      >
        Clear
      </v-btn>
    </div>

    <v-card class="flex-grow-1 pa-0" style="overflow: hidden;">
      <div v-if="loading" class="d-flex justify-center align-center fill-height pa-8">
        <v-progress-circular indeterminate color="primary" size="36" />
      </div>
      <pre
        v-else
        ref="logOutputRef"
        style="
          height: 100%;
          margin: 0;
          overflow: auto;
          background: #0a0a0a;
          color: #33ff33;
          padding: 12px;
          font-family: 'Cascadia Code', 'Fira Code', monospace;
          font-size: 12px;
          line-height: 1.4;
          white-space: pre-wrap;
          word-break: break-all;
        "
      >{{ logContent }}</pre>
    </v-card>
  </div>
</template>
