<script setup lang="ts">
import type { GameServer } from '../types'

const props = defineProps<{
  server: GameServer
}>()

const emit = defineEmits<{
  click: []
  delete: []
  logs: []
}>()

const isProvisioning = ['installing', 'downloading', 'installing_oxide', 'configuring'].includes(props.server.provisioningStatus)
const isError = props.server.provisioningStatus === 'error'

function statusLabel(status: string): string {
  switch (status) {
    case 'installing': return 'Installing LinuxGSM...'
    case 'downloading': return 'Downloading server files...'
    case 'installing_oxide': return 'Installing Oxide...'
    case 'configuring': return 'Configuring...'
    case 'error': return 'Provisioning failed'
    default: return ''
  }
}

function statusColor(): string {
  if (isError) return '#ef4444'
  if (isProvisioning) return '#f59e0b'
  if (props.server.online) return '#10b981'
  return '#64748b'
}

function handleClick() {
  if (isProvisioning || isError) {
    emit('logs')
  } else {
    emit('click')
  }
}
</script>

<template>
  <v-card
    class="server-card fill-height d-flex flex-column"
    style="cursor: pointer;"
    @click="handleClick"
  >
    <v-card-text class="flex-grow-1 pa-4">
      <div class="d-flex align-center mb-3">
        <v-icon :color="statusColor()" size="10" class="mr-2">mdi-circle</v-icon>
        <span class="text-body-1 font-weight-medium" style="color: #e2e8f0;">{{ server.name }}</span>
        <v-spacer />
        <v-chip
          :color="server.serverType === 'modded' ? 'purple' : 'info'"
          size="x-small"
          variant="tonal"
        >
          {{ server.serverType }}
        </v-chip>
      </div>

      <template v-if="isProvisioning">
        <v-progress-linear indeterminate color="warning" class="mb-2" rounded />
        <div class="text-caption text-medium-emphasis">{{ statusLabel(server.provisioningStatus) }}</div>
        <div class="text-caption text-medium-emphasis mt-1" style="opacity: 0.6;">Click to view logs</div>
      </template>

      <template v-else-if="isError">
        <div class="text-caption" style="color: #ef4444;">{{ statusLabel(server.provisioningStatus) }}</div>
        <div class="text-caption text-medium-emphasis mt-1" style="opacity: 0.6;">Click to view logs</div>
      </template>

      <template v-else>
        <div class="d-flex align-center mb-2" v-if="server.online">
          <v-icon size="14" color="medium-emphasis" class="mr-1">mdi-account-group</v-icon>
          <span class="text-body-2" style="color: #e2e8f0;">{{ server.players ?? 0 }} / {{ server.maxPlayers }}</span>
        </div>
        <div v-else class="text-caption text-medium-emphasis mb-2">Offline</div>

        <div class="d-flex flex-wrap ga-1">
          <v-chip size="x-small" variant="text" class="text-medium-emphasis">
            <v-icon start size="10">mdi-lan</v-icon>
            {{ server.gamePort }}
          </v-chip>
          <v-chip size="x-small" variant="text" class="text-medium-emphasis">
            <v-icon start size="10">mdi-earth</v-icon>
            {{ server.worldSize }}
          </v-chip>
          <v-chip size="x-small" variant="text" class="text-medium-emphasis">
            <v-icon start size="10">mdi-seed</v-icon>
            {{ server.seed }}
          </v-chip>
        </div>
      </template>
    </v-card-text>

    <v-card-actions v-if="server.source === 'dynamic' && !isProvisioning" class="pa-2 pt-0">
      <v-spacer />
      <v-btn
        icon="mdi-delete"
        size="x-small"
        variant="text"
        color="error"
        @click.stop="emit('delete')"
      />
    </v-card-actions>
  </v-card>
</template>

<style scoped>
.server-card {
  transition: border-color 0.2s, box-shadow 0.2s;
  border: 1px solid rgba(255,255,255,0.06);
}
.server-card:hover {
  border-color: rgba(255,255,255,0.12);
  box-shadow: 0 0 0 1px rgba(255,255,255,0.04);
}
</style>
