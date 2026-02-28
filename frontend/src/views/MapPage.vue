<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { serverApi } from '../services/api'
import { useRoute } from 'vue-router'
import type { PlayerPosition } from '../types'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const canvasRef = ref<HTMLCanvasElement | null>(null)
const loading = ref(true)
const mapError = ref('')

const seed = ref(0)
const worldSize = ref(4000)
const imageUrl = ref('')
const players = ref<PlayerPosition[]>([])
const hasPositionData = ref(false)

let mapImage: HTMLImageElement | null = null
let animFrame: number | null = null
let pollTimer: ReturnType<typeof setInterval> | null = null

// Pan and zoom state
const zoom = ref(1)
const panX = ref(0)
const panY = ref(0)
let isDragging = false
let dragStartX = 0
let dragStartY = 0

async function fetchMapInfo() {
  if (!serverId.value) return
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ seed: number; worldSize: number; imageUrl: string }>('/map')
    seed.value = res.data.seed
    worldSize.value = res.data.worldSize
    imageUrl.value = res.data.imageUrl
    await loadMapImage()
  } catch (e: any) {
    mapError.value = 'Failed to load map info'
  } finally {
    loading.value = false
  }
}

async function fetchPositions() {
  if (!serverId.value) return
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ players: PlayerPosition[] }>('/positions')
    players.value = res.data.players ?? []
    hasPositionData.value = players.value.length > 0
    render()
  } catch {
    // Silent
  }
}

function loadMapImage(): Promise<void> {
  return new Promise((resolve) => {
    if (!imageUrl.value) {
      resolve()
      return
    }
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      mapImage = img
      fitToCanvas()
      render()
      resolve()
    }
    img.onerror = () => {
      mapError.value = 'Failed to load map image. The map may not be available on RustMaps yet.'
      resolve()
    }
    img.src = imageUrl.value
  })
}

function fitToCanvas() {
  const canvas = canvasRef.value
  if (!canvas || !mapImage) return

  const containerWidth = canvas.parentElement?.clientWidth ?? 800
  const containerHeight = canvas.parentElement?.clientHeight ?? 600
  canvas.width = containerWidth
  canvas.height = containerHeight

  const scaleX = containerWidth / mapImage.width
  const scaleY = containerHeight / mapImage.height
  zoom.value = Math.min(scaleX, scaleY) * 0.95
  panX.value = (containerWidth - mapImage.width * zoom.value) / 2
  panY.value = (containerHeight - mapImage.height * zoom.value) / 2
}

function render() {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#0a0a0b'
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  if (mapImage) {
    ctx.save()
    ctx.translate(panX.value, panY.value)
    ctx.scale(zoom.value, zoom.value)
    ctx.drawImage(mapImage, 0, 0)

    // Draw player positions
    if (players.value.length > 0 && mapImage) {
      const half = worldSize.value / 2
      const imgW = mapImage.width
      const imgH = mapImage.height

      for (const p of players.value) {
        // Convert world coordinates to image coordinates
        // World origin is center, X right, Z up (Y in game is height)
        const imgX = ((p.x + half) / worldSize.value) * imgW
        const imgY = ((half - p.z) / worldSize.value) * imgH

        // Draw dot
        ctx.beginPath()
        ctx.arc(imgX, imgY, 4 / zoom.value, 0, Math.PI * 2)
        ctx.fillStyle = '#3b82f6'
        ctx.fill()
        ctx.strokeStyle = '#ffffff'
        ctx.lineWidth = 1.5 / zoom.value
        ctx.stroke()

        // Draw name
        ctx.font = `${11 / zoom.value}px sans-serif`
        ctx.fillStyle = '#ffffff'
        ctx.textAlign = 'center'
        ctx.fillText(p.displayName, imgX, imgY - 8 / zoom.value)
      }
    }

    ctx.restore()
  }
}

function handleWheel(e: WheelEvent) {
  e.preventDefault()
  const canvas = canvasRef.value
  if (!canvas) return

  const rect = canvas.getBoundingClientRect()
  const mouseX = e.clientX - rect.left
  const mouseY = e.clientY - rect.top

  const oldZoom = zoom.value
  const delta = e.deltaY > 0 ? 0.9 : 1.1
  zoom.value = Math.max(0.1, Math.min(10, zoom.value * delta))

  // Zoom towards mouse cursor
  panX.value = mouseX - (mouseX - panX.value) * (zoom.value / oldZoom)
  panY.value = mouseY - (mouseY - panY.value) * (zoom.value / oldZoom)

  render()
}

function handleMouseDown(e: MouseEvent) {
  isDragging = true
  dragStartX = e.clientX - panX.value
  dragStartY = e.clientY - panY.value
}

function handleMouseMove(e: MouseEvent) {
  if (!isDragging) return
  panX.value = e.clientX - dragStartX
  panY.value = e.clientY - dragStartY
  render()
}

function handleMouseUp() {
  isDragging = false
}

function zoomIn() {
  zoom.value = Math.min(10, zoom.value * 1.2)
  render()
}

function zoomOut() {
  zoom.value = Math.max(0.1, zoom.value / 1.2)
  render()
}

function resetView() {
  fitToCanvas()
  render()
}

onMounted(async () => {
  await fetchMapInfo()
  fetchPositions()
  pollTimer = setInterval(fetchPositions, 5000)

  window.addEventListener('resize', () => {
    const canvas = canvasRef.value
    if (canvas && canvas.parentElement) {
      canvas.width = canvas.parentElement.clientWidth
      canvas.height = canvas.parentElement.clientHeight
      render()
    }
  })
})

onUnmounted(() => {
  if (animFrame) cancelAnimationFrame(animFrame)
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div class="d-flex flex-column" style="height: calc(100vh - 100px);">
    <div class="d-flex align-center mb-3 flex-wrap ga-2">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Map</div>
      <v-chip size="small" variant="tonal" color="info" v-if="seed">
        <v-icon start size="12">mdi-seed</v-icon>
        {{ seed }}
      </v-chip>
      <v-chip size="small" variant="tonal" color="info" v-if="worldSize">
        <v-icon start size="12">mdi-earth</v-icon>
        {{ worldSize }}
      </v-chip>
      <v-chip size="small" variant="tonal" :color="hasPositionData ? 'success' : 'default'">
        <v-icon start size="12">mdi-account-group</v-icon>
        {{ players.length }} players
      </v-chip>
      <v-spacer />
      <v-btn size="small" variant="tonal" color="primary" icon="mdi-plus" @click="zoomIn" />
      <v-btn size="small" variant="tonal" color="primary" icon="mdi-minus" @click="zoomOut" />
      <v-btn size="small" variant="tonal" color="primary" icon="mdi-fit-to-screen" @click="resetView" />
    </div>

    <v-card class="flex-grow-1 pa-0" style="overflow: hidden; position: relative;">
      <div v-if="loading" class="d-flex justify-center align-center fill-height">
        <v-progress-circular indeterminate color="primary" size="48" />
      </div>

      <div v-else-if="mapError" class="d-flex flex-column justify-center align-center fill-height text-center pa-8">
        <v-icon size="64" color="medium-emphasis" class="mb-4">mdi-map-marker-off</v-icon>
        <div class="text-body-1 text-medium-emphasis mb-2">{{ mapError }}</div>
        <div class="text-caption text-medium-emphasis">Seed: {{ seed }}, Size: {{ worldSize }}</div>
      </div>

      <div v-else style="width: 100%; height: 100%;">
        <canvas
          ref="canvasRef"
          style="width: 100%; height: 100%; display: block;"
          @wheel="handleWheel"
          @mousedown="handleMouseDown"
          @mousemove="handleMouseMove"
          @mouseup="handleMouseUp"
          @mouseleave="handleMouseUp"
        />
      </div>

      <div
        v-if="!loading && !mapError && !hasPositionData"
        style="position: absolute; bottom: 16px; left: 16px; right: 16px;"
      >
        <v-alert type="info" variant="tonal" density="compact">
          Install the position tracking Oxide plugin on your server for live player positions.
        </v-alert>
      </div>
    </v-card>

    <!-- Player list -->
    <v-card v-if="players.length > 0" class="mt-3 pa-3">
      <div class="text-caption text-medium-emphasis mb-2">Players on map</div>
      <div class="d-flex flex-wrap ga-2">
        <v-chip
          v-for="p in players"
          :key="p.steamId"
          size="small"
          variant="tonal"
          color="primary"
        >
          <v-icon start size="10">mdi-account</v-icon>
          {{ p.displayName }}
          <span class="text-medium-emphasis ml-1">({{ p.x.toFixed(0) }}, {{ p.z.toFixed(0) }})</span>
        </v-chip>
      </div>
    </v-card>
  </div>
</template>
