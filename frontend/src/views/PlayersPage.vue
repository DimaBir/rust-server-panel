<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { serverApi } from '../services/api'
import { useRoute } from 'vue-router'
import type { Player } from '../types'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const players = ref<Player[]>([])
const loading = ref(true)
const search = ref('')

const kickDialog = ref(false)
const banDialog = ref(false)
const moderatorDialog = ref(false)
const giveItemDialog = ref(false)
const selectedPlayer = ref<Player | null>(null)
const actionReason = ref('')

const giveItemName = ref('')
const giveItemAmount = ref(1)
const moderatorAction = ref<'add' | 'remove'>('add')

let pollTimer: ReturnType<typeof setInterval> | null = null

const onlineHeaders = [
  { title: 'Name', key: 'displayName' },
  { title: 'Steam ID', key: 'steamId' },
  { title: 'Ping', key: 'ping' },
  { title: 'Health', key: 'health' },
  { title: 'Connected', key: 'connectedSeconds' },
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
  if (!serverId.value) return
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ players: Player[] }>('/players')
    players.value = res.data.players ?? []
  } catch {
    // Silent poll
  } finally {
    loading.value = false
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

function openModerator(player: Player, action: 'add' | 'remove') {
  selectedPlayer.value = player
  moderatorAction.value = action
  moderatorDialog.value = true
}

function openGiveItem(player: Player) {
  selectedPlayer.value = player
  giveItemName.value = ''
  giveItemAmount.value = 1
  giveItemDialog.value = true
}

async function confirmKick() {
  if (!selectedPlayer.value) return
  try {
    const api = serverApi(serverId.value)
    await api.post('/players/kick', {
      steamId: selectedPlayer.value.steamId,
      reason: actionReason.value || 'Kicked by admin',
    })
    kickDialog.value = false
    await fetchPlayers()
  } catch { /* interceptor */ }
}

async function confirmBan() {
  if (!selectedPlayer.value) return
  try {
    const api = serverApi(serverId.value)
    await api.post('/players/ban', {
      steamId: selectedPlayer.value.steamId,
      reason: actionReason.value || 'Banned by admin',
    })
    banDialog.value = false
    await fetchPlayers()
  } catch { /* interceptor */ }
}

async function confirmModerator() {
  if (!selectedPlayer.value) return
  try {
    const api = serverApi(serverId.value)
    if (moderatorAction.value === 'add') {
      await api.post('/players/moderator', {
        steamId: selectedPlayer.value.steamId,
        displayName: selectedPlayer.value.displayName,
      })
    } else {
      await api.post('/players/remove-moderator', {
        steamId: selectedPlayer.value.steamId,
      })
    }
    moderatorDialog.value = false
  } catch { /* interceptor */ }
}

async function confirmGiveItem() {
  if (!selectedPlayer.value || !giveItemName.value) return
  try {
    const api = serverApi(serverId.value)
    await api.post('/players/give', {
      steamId: selectedPlayer.value.steamId,
      item: giveItemName.value,
      amount: giveItemAmount.value,
    })
    giveItemDialog.value = false
  } catch { /* interceptor */ }
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

onMounted(() => {
  fetchPlayers()
  pollTimer = setInterval(fetchPlayers, 10000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div>
    <div class="text-h6 font-weight-medium mb-4" style="color: #e2e8f0;">Player Management</div>

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
              variant="tonal"
            >
              {{ item.health.toFixed(0) }}
            </v-chip>
          </template>
          <template #item.ping="{ item }">
            <span class="text-medium-emphasis">{{ item.ping }}ms</span>
          </template>
          <template #item.connectedSeconds="{ item }">
            <span class="text-medium-emphasis">{{ formatTime(item.connectedSeconds) }}</span>
          </template>
          <template #item.actions="{ item }">
            <v-menu>
              <template #activator="{ props }">
                <v-btn icon="mdi-dots-vertical" size="small" variant="text" v-bind="props" />
              </template>
              <v-list density="compact">
                <v-list-item prepend-icon="mdi-account-remove" title="Kick" @click="openKick(item)" />
                <v-list-item prepend-icon="mdi-cancel" title="Ban" @click="openBan(item)" />
                <v-divider />
                <v-list-item prepend-icon="mdi-shield-account" title="Make Moderator" @click="openModerator(item, 'add')" />
                <v-list-item prepend-icon="mdi-shield-off" title="Remove Moderator" @click="openModerator(item, 'remove')" />
                <v-divider />
                <v-list-item prepend-icon="mdi-gift" title="Give Item" @click="openGiveItem(item)" />
              </v-list>
            </v-menu>
          </template>
          <template #no-data>
            <div class="text-center pa-8 text-medium-emphasis">No players online</div>
          </template>
        </v-data-table>
      </v-card-text>
    </v-card>

    <!-- Kick Dialog -->
    <v-dialog v-model="kickDialog" max-width="450">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Kick Player</v-card-title>
        <v-card-text>
          <p class="mb-3">Kick <strong>{{ selectedPlayer?.displayName }}</strong>?</p>
          <v-text-field v-model="actionReason" label="Reason (optional)" hide-details />
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
        <v-card-title class="text-h6 font-weight-medium">Ban Player</v-card-title>
        <v-card-text>
          <p class="mb-3">Ban <strong>{{ selectedPlayer?.displayName }}</strong>?</p>
          <v-text-field v-model="actionReason" label="Reason (optional)" hide-details />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="banDialog = false">Cancel</v-btn>
          <v-btn color="error" variant="flat" @click="confirmBan">Ban</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Moderator Dialog -->
    <v-dialog v-model="moderatorDialog" max-width="450">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">
          {{ moderatorAction === 'add' ? 'Make Moderator' : 'Remove Moderator' }}
        </v-card-title>
        <v-card-text>
          <p>
            {{ moderatorAction === 'add' ? 'Grant moderator privileges to' : 'Remove moderator privileges from' }}
            <strong>{{ selectedPlayer?.displayName }}</strong>?
          </p>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="moderatorDialog = false">Cancel</v-btn>
          <v-btn
            :color="moderatorAction === 'add' ? 'primary' : 'warning'"
            variant="flat"
            @click="confirmModerator"
          >
            {{ moderatorAction === 'add' ? 'Make Moderator' : 'Remove Moderator' }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Give Item Dialog -->
    <v-dialog v-model="giveItemDialog" max-width="450">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Give Item</v-card-title>
        <v-card-text>
          <p class="mb-3">Give item to <strong>{{ selectedPlayer?.displayName }}</strong></p>
          <v-text-field
            v-model="giveItemName"
            label="Item shortname"
            placeholder="e.g. rifle.ak"
            hide-details
            class="mb-3"
          />
          <v-text-field
            v-model.number="giveItemAmount"
            label="Amount"
            type="number"
            min="1"
            hide-details
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="giveItemDialog = false">Cancel</v-btn>
          <v-btn color="primary" variant="flat" :disabled="!giveItemName" @click="confirmGiveItem">Give</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
