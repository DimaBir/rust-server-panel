<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '../services/api'

const tab = ref('servercfg')
const loading = ref(true)
const saving = ref(false)

// server.cfg
const serverCfg = ref('')
const cfgModified = ref(false)

// Startup Parameters
const startupParams = ref({
  serverName: '',
  mapSize: '4000',
  seed: '',
  maxPlayers: '100',
  rconPassword: '',
  rconPort: '28016',
  serverPort: '28015',
  serverIp: '0.0.0.0',
  tickRate: '30',
  saveInterval: '300',
})
const paramsModified = ref(false)

async function fetchServerCfg() {
  loading.value = true
  try {
    const res = await api.get<{ content: string }>('/files/read', {
      params: { path: 'server/server.cfg' },
    })
    serverCfg.value = res.data.content ?? ''
    cfgModified.value = false
  } catch {
    serverCfg.value = '# Could not load server.cfg\n'
  } finally {
    loading.value = false
  }
}

async function saveServerCfg() {
  saving.value = true
  try {
    await api.put('/files/write', {
      path: 'server/server.cfg',
      content: serverCfg.value,
    })
    cfgModified.value = false
  } catch {
    // Error handled by interceptor
  } finally {
    saving.value = false
  }
}

async function fetchStartupParams() {
  try {
    const res = await api.get<{ params: typeof startupParams.value }>('/server/startup-params')
    if (res.data.params) {
      startupParams.value = { ...startupParams.value, ...res.data.params }
    }
    paramsModified.value = false
  } catch {
    // Use defaults
  }
}

async function saveStartupParams() {
  saving.value = true
  try {
    await api.put('/server/startup-params', startupParams.value)
    paramsModified.value = false
  } catch {
    // Error handled by interceptor
  } finally {
    saving.value = false
  }
}

function markParamsModified() {
  paramsModified.value = true
}

onMounted(() => {
  fetchServerCfg()
  fetchStartupParams()
})
</script>

<template>
  <div>
    <div class="text-h5 font-weight-bold mb-4">Server Configuration</div>

    <v-tabs v-model="tab" color="primary" class="mb-4">
      <v-tab value="servercfg">
        <v-icon start>mdi-file-cog</v-icon>
        server.cfg
      </v-tab>
      <v-tab value="startup">
        <v-icon start>mdi-rocket-launch</v-icon>
        Startup Parameters
      </v-tab>
    </v-tabs>

    <v-window v-model="tab">
      <!-- server.cfg Tab -->
      <v-window-item value="servercfg">
        <v-card>
          <v-card-title class="d-flex align-center">
            <v-icon class="mr-2">mdi-file-document-edit</v-icon>
            server.cfg
            <v-chip v-if="cfgModified" size="x-small" color="warning" class="ml-2">Modified</v-chip>
            <v-spacer />
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              prepend-icon="mdi-content-save"
              :loading="saving"
              :disabled="!cfgModified"
              @click="saveServerCfg"
            >
              Save
            </v-btn>
          </v-card-title>
          <v-card-text>
            <div v-if="loading" class="text-center pa-8">
              <v-progress-circular indeterminate color="primary" size="36" />
            </div>
            <textarea
              v-else
              v-model="serverCfg"
              spellcheck="false"
              style="
                width: 100%;
                height: calc(100vh - 320px);
                min-height: 400px;
                background: #0a0a0a;
                color: #e0e0e0;
                border: 1px solid #333;
                border-radius: 4px;
                outline: none;
                padding: 12px;
                font-family: 'Cascadia Code', 'Fira Code', monospace;
                font-size: 13px;
                line-height: 1.5;
                resize: vertical;
                tab-size: 2;
              "
              @input="cfgModified = true"
            />
          </v-card-text>
        </v-card>
      </v-window-item>

      <!-- Startup Parameters Tab -->
      <v-window-item value="startup">
        <v-card>
          <v-card-title class="d-flex align-center">
            <v-icon class="mr-2">mdi-rocket-launch</v-icon>
            Startup Parameters
            <v-chip v-if="paramsModified" size="x-small" color="warning" class="ml-2">Modified</v-chip>
            <v-spacer />
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              prepend-icon="mdi-content-save"
              :loading="saving"
              :disabled="!paramsModified"
              @click="saveStartupParams"
            >
              Save
            </v-btn>
          </v-card-title>
          <v-card-text>
            <v-row>
              <v-col cols="12" md="6">
                <v-text-field
                  v-model="startupParams.serverName"
                  label="Server Name"
                  prepend-inner-icon="mdi-label"
                  hint="The name shown in the server browser"
                  persistent-hint
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" md="6">
                <v-text-field
                  v-model="startupParams.serverIp"
                  label="Server IP"
                  prepend-inner-icon="mdi-ip-network"
                  hint="Bind address (0.0.0.0 for all interfaces)"
                  persistent-hint
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="3">
                <v-text-field
                  v-model="startupParams.serverPort"
                  label="Server Port"
                  prepend-inner-icon="mdi-network"
                  type="number"
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="3">
                <v-text-field
                  v-model="startupParams.maxPlayers"
                  label="Max Players"
                  prepend-inner-icon="mdi-account-group"
                  type="number"
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="3">
                <v-text-field
                  v-model="startupParams.mapSize"
                  label="Map Size"
                  prepend-inner-icon="mdi-map"
                  type="number"
                  hint="1000 - 6000"
                  persistent-hint
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="3">
                <v-text-field
                  v-model="startupParams.seed"
                  label="Map Seed"
                  prepend-inner-icon="mdi-dice-multiple"
                  hint="Leave blank for random"
                  persistent-hint
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="4">
                <v-text-field
                  v-model="startupParams.rconPassword"
                  label="RCON Password"
                  prepend-inner-icon="mdi-lock"
                  type="password"
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="4">
                <v-text-field
                  v-model="startupParams.rconPort"
                  label="RCON Port"
                  prepend-inner-icon="mdi-network"
                  type="number"
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="4">
                <v-text-field
                  v-model="startupParams.tickRate"
                  label="Tick Rate"
                  prepend-inner-icon="mdi-speedometer"
                  type="number"
                  @update:model-value="markParamsModified"
                />
              </v-col>
              <v-col cols="12" sm="6" md="4">
                <v-text-field
                  v-model="startupParams.saveInterval"
                  label="Save Interval (seconds)"
                  prepend-inner-icon="mdi-content-save-clock"
                  type="number"
                  hint="Auto-save interval in seconds"
                  persistent-hint
                  @update:model-value="markParamsModified"
                />
              </v-col>
            </v-row>
          </v-card-text>
        </v-card>
      </v-window-item>
    </v-window>
  </div>
</template>
