<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { serverApi } from '../services/api'
import api from '../services/api'
import { useServerStore } from '../stores/server'
import type { Plugin } from '../types'

const serverStore = useServerStore()
const tab = ref('installed')
const loading = ref(true)
const plugins = ref<Plugin[]>([])

const configDialog = ref(false)
const configPlugin = ref<Plugin | null>(null)
const configContent = ref('')
const configSaving = ref(false)
const deleteDialog = ref(false)
const deleteTarget = ref<Plugin | null>(null)
const uploadInput = ref<HTMLInputElement | null>(null)

const umodSearch = ref('')
const umodResults = ref<any[]>([])
const umodLoading = ref(false)
const installingPlugin = ref<string | null>(null)

const activeServerId = computed(() => serverStore.activeServerId ?? '')

const installedHeaders = [
  { title: 'Plugin', key: 'name' },
  { title: 'File', key: 'filename' },
  { title: 'Size', key: 'size' },
  { title: 'Modified', key: 'modified' },
  { title: 'Config', key: 'hasConfig' },
  { title: 'Actions', key: 'actions', sortable: false },
]

async function fetchPlugins() {
  if (!activeServerId.value) return
  loading.value = true
  try {
    const sApi = serverApi(activeServerId.value)
    const res = await sApi.get<Plugin[]>('/plugins')
    plugins.value = res.data ?? []
  } catch { plugins.value = [] }
  finally { loading.value = false }
}

async function openConfig(plugin: Plugin) {
  try {
    const sApi = serverApi(activeServerId.value)
    const res = await sApi.get<{ config: any }>(`/plugins/${encodeURIComponent(plugin.name)}/config`)
    configPlugin.value = plugin
    configContent.value = typeof res.data.config === 'string' ? res.data.config : JSON.stringify(res.data.config, null, 2)
    configDialog.value = true
  } catch { /* interceptor */ }
}

async function saveConfig() {
  if (!configPlugin.value) return
  configSaving.value = true
  try {
    const sApi = serverApi(activeServerId.value)
    let parsed
    try { parsed = JSON.parse(configContent.value) } catch { parsed = configContent.value }
    await sApi.put(`/plugins/${encodeURIComponent(configPlugin.value.name)}/config`, parsed)
    configDialog.value = false
  } catch { /* interceptor */ }
  finally { configSaving.value = false }
}

async function reloadPlugin(plugin: Plugin) {
  try {
    const sApi = serverApi(activeServerId.value)
    await sApi.post(`/plugins/${encodeURIComponent(plugin.name)}/reload`)
    await fetchPlugins()
  } catch { /* interceptor */ }
}

function confirmDelete(plugin: Plugin) {
  deleteTarget.value = plugin
  deleteDialog.value = true
}

async function executeDelete() {
  if (!deleteTarget.value) return
  try {
    const sApi = serverApi(activeServerId.value)
    await sApi.delete(`/plugins/${encodeURIComponent(deleteTarget.value.name)}`)
    deleteDialog.value = false
    deleteTarget.value = null
    await fetchPlugins()
  } catch { /* interceptor */ }
}

function triggerUpload() { uploadInput.value?.click() }

async function handleUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const formData = new FormData()
  formData.append('file', file)
  try {
    const sApi = serverApi(activeServerId.value)
    await sApi.post('/plugins/upload', formData, { headers: { 'Content-Type': 'multipart/form-data' } })
    await fetchPlugins()
  } catch { /* interceptor */ }
  finally { input.value = '' }
}

async function searchUMod() {
  if (!umodSearch.value.trim()) return
  umodLoading.value = true
  try {
    const res = await api.get('/plugins/umod/search', { params: { q: umodSearch.value } })
    umodResults.value = res.data?.data ?? []
  } catch { umodResults.value = [] }
  finally { umodLoading.value = false }
}

async function installUModPlugin(plugin: any) {
  installingPlugin.value = plugin.name
  try {
    const sApi = serverApi(activeServerId.value)
    await sApi.post('/plugins/umod/install', {
      url: plugin.download_url || plugin.downloadUrl,
      filename: `${plugin.name}.cs`,
    })
    await fetchPlugins()
  } catch { /* interceptor */ }
  finally { installingPlugin.value = null }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  const kb = bytes / 1024
  if (kb < 1024) return kb.toFixed(1) + ' KB'
  return (kb / 1024).toFixed(1) + ' MB'
}

function formatDate(dateStr: string): string {
  if (!dateStr) return ''
  return new Date(dateStr).toLocaleString()
}

watch(() => serverStore.activeServerId, () => {
  loading.value = true
  fetchPlugins()
})

onMounted(() => { fetchPlugins() })
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Plugin Manager</div>
      <v-spacer />
      <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-upload" @click="triggerUpload">Upload Plugin</v-btn>
      <input ref="uploadInput" type="file" accept=".cs" style="display: none;" @change="handleUpload" />
    </div>

    <v-tabs v-model="tab" color="primary" class="mb-4">
      <v-tab value="installed">Installed <v-chip size="x-small" class="ml-2" color="primary" variant="tonal">{{ plugins.length }}</v-chip></v-tab>
      <v-tab value="umod">Browse uMod</v-tab>
    </v-tabs>

    <v-window v-model="tab">
      <v-window-item value="installed">
        <v-card>
          <v-card-text>
            <v-data-table :headers="installedHeaders" :items="plugins" :loading="loading" item-key="name" class="elevation-0" density="comfortable">
              <template #item.size="{ item }">{{ formatSize(item.size) }}</template>
              <template #item.modified="{ item }">{{ formatDate(item.modified) }}</template>
              <template #item.hasConfig="{ item }">
                <v-icon v-if="item.hasConfig" color="success" size="small">mdi-check-circle</v-icon>
                <v-icon v-else color="medium-emphasis" size="small">mdi-minus-circle-outline</v-icon>
              </template>
              <template #item.actions="{ item }">
                <v-btn v-if="item.hasConfig" icon="mdi-cog" size="small" variant="text" color="medium-emphasis" @click="openConfig(item)" />
                <v-btn icon="mdi-reload" size="small" variant="text" color="medium-emphasis" @click="reloadPlugin(item)" />
                <v-btn icon="mdi-delete" size="small" variant="text" color="error" @click="confirmDelete(item)" />
              </template>
              <template #no-data><div class="text-center pa-8 text-medium-emphasis">No plugins installed</div></template>
            </v-data-table>
          </v-card-text>
        </v-card>
      </v-window-item>

      <v-window-item value="umod">
        <v-card class="mb-4">
          <v-card-text>
            <v-text-field v-model="umodSearch" prepend-inner-icon="mdi-magnify" label="Search uMod plugins..." hide-details @keydown.enter="searchUMod">
              <template #append><v-btn color="primary" variant="flat" :loading="umodLoading" @click="searchUMod">Search</v-btn></template>
            </v-text-field>
          </v-card-text>
        </v-card>
        <v-row v-if="umodLoading"><v-col cols="12" class="text-center py-8"><v-progress-circular indeterminate color="primary" size="36" /></v-col></v-row>
        <v-row v-else-if="umodResults.length === 0 && umodSearch"><v-col cols="12"><div class="text-center pa-8 text-medium-emphasis">No results found</div></v-col></v-row>
        <v-row v-else>
          <v-col v-for="plugin in umodResults" :key="plugin.name" cols="12" sm="6" md="4">
            <v-card class="fill-height d-flex flex-column">
              <v-card-title class="text-body-1 font-weight-medium">{{ plugin.title || plugin.name }}</v-card-title>
              <v-card-subtitle>by {{ plugin.author }} &bull; v{{ plugin.version }}</v-card-subtitle>
              <v-card-text class="flex-grow-1">
                <div class="text-body-2 text-medium-emphasis mb-2" style="display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;">{{ plugin.description }}</div>
              </v-card-text>
              <v-card-actions>
                <v-spacer />
                <v-btn color="primary" variant="tonal" size="small" prepend-icon="mdi-download" :loading="installingPlugin === plugin.name" @click="installUModPlugin(plugin)">Install</v-btn>
              </v-card-actions>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>
    </v-window>

    <v-dialog v-model="configDialog" max-width="700">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">{{ configPlugin?.name }} - Config</v-card-title>
        <v-card-text>
          <textarea v-model="configContent" spellcheck="false" style="width: 100%; height: 400px; background: #0a0a0b; color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; outline: none; padding: 12px; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 13px; line-height: 1.5; resize: vertical; tab-size: 2;" />
        </v-card-text>
        <v-card-actions><v-spacer /><v-btn variant="text" @click="configDialog = false">Cancel</v-btn><v-btn color="primary" variant="flat" :loading="configSaving" @click="saveConfig">Save Config</v-btn></v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Delete Plugin</v-card-title>
        <v-card-text>Are you sure you want to delete <strong>{{ deleteTarget?.name }}</strong>?</v-card-text>
        <v-card-actions><v-spacer /><v-btn variant="text" @click="deleteDialog = false">Cancel</v-btn><v-btn color="error" variant="flat" @click="executeDelete">Delete</v-btn></v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
