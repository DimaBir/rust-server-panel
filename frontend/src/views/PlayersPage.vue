<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import api from '../services/api'
import type { Player } from '../types'

const tab = ref('online')
const players = ref<Player[]>([])
const bannedPlayers = ref<{ steamId: string; reason: string }[]>([])
const loading = ref(true)
const search = ref('')

const kickDialog = ref(false)
const banDialog = ref(false)
const selectedPlayer = ref<Player | null>(null)
const actionReason = ref('')

let pollTimer: ReturnType<typeof setInterval> | null = null

const onlineHeaders = [
  { title: 'Name', key: 'displayName' },
  { title: 'Steam ID', key: 'steamId' },
  { title: 'Ping', key: 'ping' },
  { title: 'Health', key: 'health' },
  { title: 'Connected', key: 'connectedSeconds' },
  { title: 'Actions', key: 'actions', sortable: false },
]

const banHeaders = [
  { title: 'Steam ID', key: 'steamId' },
  { title: 'Reason', key: 'reason' },
  { title: 'Actions', key: 'actions', sortable: false },
]

const filteredPlayers = computed(() => {
  if (!search.value) return players.value
  const q = search.value.toLowerCase()
  return players.value.filter(
    (p) => p.displayName.toLowerCase().includes(q) || p.steamId.includes(q)
  )
})

async function fetchPlayers() {
  try {
    const res = await api.get<{ players: Player[] }>('/monitor/game')
    players.value = res.data.players ?? []
  } catch {
    // Silent poll error
  } finally {
    loading.value = false
  }
}

async function fetchBans() {
  try {
    const res = await api.get<{ bans: { steamId: string; reason: string }[] }>('/players/bans')
    bannedPlayers.value = res.data.bans ?? []
  } catch {
    // Silent error
  }
}

function openKick(player: Player) {
  selectedPlayer.value = player
  actionReason.value = ''
  kickDialog.value = true
}

function openBan(player: Player) {
  selectedPlayer.value = player
  actionReason.value = ''
  banDialog.value = true
}

async function confirmKick() {
  if (!selectedPlayer.value) return
  try {
    await api.post('/players/kick', {
      steamId: selectedPlayer.value.steamId,
      reason: actionReason.value || 'Kicked by admin',
    })
    kickDialog.value = false
    await fetchPlayers()
  } catch {
    // Error handled by interceptor
  }
}

async function confirmBan() {
  if (!selectedPlayer.value) return
  try {
    await api.post('/players/ban', {
      steamId: selectedPlayer.value.steamId,
      reason: actionReason.value || 'Banned by admin',
    })
    banDialog.value = false
    await fetchPlayers()
    await fetchBans()
  } catch {
    // Error handled by interceptor
  }
}

async function unban(steamId: string) {
  try {
    await api.post('/players/unban', { steamId })
    await fetchBans()
  } catch {
    // Error handled by interceptor
  }
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

onMounted(() => {
  fetchPlayers()
  fetchBans()
  pollTimer = setInterval(fetchPlayers, 10000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div>
    <div class="text-h5 font-weight-bold mb-4">Player Management</div>

    <v-tabs v-model="tab" color="primary" class="mb-4">
      <v-tab value="online">
        Online Players
        <v-chip size="x-small" class="ml-2" color="success">{{ players.length }}</v-chip>
      </v-tab>
      <v-tab value="bans">
        Ban List
        <v-chip size="x-small" class="ml-2" color="error">{{ bannedPlayers.length }}</v-chip>
      </v-tab>
    </v-tabs>

    <v-window v-model="tab">
      <!-- Online Players -->
      <v-window-item value="online">
        <v-card>
          <v-card-text>
            <v-text-field
              v-model="search"
              prepend-inner-icon="mdi-magnify"
              label="Search players..."
              hide-details
              class="mb-4"
            />

            <v-data-table
              :headers="onlineHeaders"
              :items="filteredPlayers"
              :loading="loading"
              item-key="steamId"
              class="elevation-0"
              density="comfortable"
            >
              <template #item.health="{ item }">
                <v-chip
                  :color="item.health > 50 ? 'success' : item.health > 20 ? 'warning' : 'error'"
                  size="small"
                >
                  {{ item.health.toFixed(0) }}
                </v-chip>
              </template>
              <template #item.ping="{ item }">
                <v-chip
                  :color="item.ping < 50 ? 'success' : item.ping < 100 ? 'warning' : 'error'"
                  size="small"
                >
                  {{ item.ping }}ms
                </v-chip>
              </template>
              <template #item.connectedSeconds="{ item }">
                {{ formatTime(item.connectedSeconds) }}
              </template>
              <template #item.actions="{ item }">
                <v-btn
                  icon="mdi-account-remove"
                  size="small"
                  variant="text"
                  color="warning"
                  title="Kick"
                  @click="openKick(item)"
                />
                <v-btn
                  icon="mdi-cancel"
                  size="small"
                  variant="text"
                  color="error"
                  title="Ban"
                  @click="openBan(item)"
                />
              </template>
              <template #no-data>
                <div class="text-center pa-4 text-grey">No players online</div>
              </template>
            </v-data-table>
          </v-card-text>
        </v-card>
      </v-window-item>

      <!-- Ban List -->
      <v-window-item value="bans">
        <v-card>
          <v-card-text>
            <v-data-table
              :headers="banHeaders"
              :items="bannedPlayers"
              item-key="steamId"
              class="elevation-0"
              density="comfortable"
            >
              <template #item.actions="{ item }">
                <v-btn
                  size="small"
                  variant="tonal"
                  color="success"
                  @click="unban(item.steamId)"
                >
                  Unban
                </v-btn>
              </template>
              <template #no-data>
                <div class="text-center pa-4 text-grey">No banned players</div>
              </template>
            </v-data-table>
          </v-card-text>
        </v-card>
      </v-window-item>
    </v-window>

    <!-- Kick Dialog -->
    <v-dialog v-model="kickDialog" max-width="450">
      <v-card>
        <v-card-title>Kick Player</v-card-title>
        <v-card-text>
          <p class="mb-3">Kick <strong>{{ selectedPlayer?.displayName }}</strong>?</p>
          <v-text-field
            v-model="actionReason"
            label="Reason (optional)"
            hide-details
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="kickDialog = false">Cancel</v-btn>
          <v-btn color="warning" variant="flat" @click="confirmKick">Kick</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Ban Dialog -->
    <v-dialog v-model="banDialog" max-width="450">
      <v-card>
        <v-card-title>Ban Player</v-card-title>
        <v-card-text>
          <p class="mb-3">Ban <strong>{{ selectedPlayer?.displayName }}</strong>?</p>
          <v-text-field
            v-model="actionReason"
            label="Reason (optional)"
            hide-details
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="banDialog = false">Cancel</v-btn>
          <v-btn color="error" variant="flat" @click="confirmBan">Ban</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
