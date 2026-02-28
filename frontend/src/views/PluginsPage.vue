<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '../services/api'
import type { Plugin, UModPlugin } from '../types'

const tab = ref('installed')
const loading = ref(true)
const plugins = ref<Plugin[]>([])

// Installed plugins
const configDialog = ref(false)
const configPlugin = ref<Plugin | null>(null)
const configContent = ref('')
const configSaving = ref(false)

const deleteDialog = ref(false)
const deleteTarget = ref<Plugin | null>(null)

const uploadInput = ref<HTMLInputElement | null>(null)

// uMod browser
const umodSearch = ref('')
const umodResults = ref<UModPlugin[]>([])
const umodLoading = ref(false)
const installingPlugin = ref<string | null>(null)

const installedHeaders = [
  { title: 'Plugin', key: 'name' },
  { title: 'File', key: 'fileName' },
  { title: 'Size', key: 'size' },
  { title: 'Modified', key: 'modifiedAt' },
  { title: 'Status', key: 'loaded' },
  { title: 'Actions', key: 'actions', sortable: false },
]

async function fetchPlugins() {
  loading.value = true
  try {
    const res = await api.get<{ plugins: Plugin[] }>('/plugins')
    plugins.value = res.data.plugins ?? []
  } catch {
    plugins.value = []
  } finally {
    loading.value = false
  }
}

async function openConfig(plugin: Plugin) {
  try {
    const res = await api.get<{ config: string }>(`/plugins/${encodeURIComponent(plugin.name)}/config`)
    configPlugin.value = plugin
    configContent.value = res.data.config ?? ''
    configDialog.value = true
  } catch {
    // Error handled by interceptor
  }
}

async function saveConfig() {
  if (!configPlugin.value) return
  configSaving.value = true
  try {
    await api.put(`/plugins/${encodeURIComponent(configPlugin.value.name)}/config`, {
      config: configContent.value,
    })
    configDialog.value = false
  } catch {
    // Error handled by interceptor
  } finally {
    configSaving.value = false
  }
}

async function reloadPlugin(plugin: Plugin) {
  try {
    await api.post(`/plugins/${encodeURIComponent(plugin.name)}/reload`)
    await fetchPlugins()
  } catch {
    // Error handled by interceptor
  }
}

function confirmDelete(plugin: Plugin) {
  deleteTarget.value = plugin
  deleteDialog.value = true
}

async function executeDelete() {
  if (!deleteTarget.value) return
  try {
    await api.delete(`/plugins/${encodeURIComponent(deleteTarget.value.name)}`)
    deleteDialog.value = false
    deleteTarget.value = null
    await fetchPlugins()
  } catch {
    // Error handled by interceptor
  }
}

function triggerUpload() {
  uploadInput.value?.click()
}

async function handleUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  const formData = new FormData()
  formData.append('file', file)

  try {
    await api.post('/plugins/upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
    await fetchPlugins()
  } catch {
    // Error handled by interceptor
  } finally {
    input.value = ''
  }
}

async function searchUMod() {
  if (!umodSearch.value.trim()) return
  umodLoading.value = true
  try {
    const res = await api.get<{ plugins: UModPlugin[] }>('/plugins/umod/search', {
      params: { query: umodSearch.value },
    })
    umodResults.value = res.data.plugins ?? []
  } catch {
    umodResults.value = []
  } finally {
    umodLoading.value = false
  }
}

async function installUModPlugin(plugin: UModPlugin) {
  installingPlugin.value = plugin.name
  try {
    await api.post('/plugins/umod/install', {
      name: plugin.name,
      downloadUrl: plugin.downloadUrl,
    })
    await fetchPlugins()
  } catch {
    // Error handled by interceptor
  } finally {
    installingPlugin.value = null
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  const kb = bytes / 1024
  if (kb < 1024) return kb.toFixed(1) + ' KB'
  const mb = kb / 1024
  return mb.toFixed(1) + ' MB'
}

function formatDate(dateStr: string): string {
  if (!dateStr) return ''
  return new Date(dateStr).toLocaleString()
}

onMounted(() => {
  fetchPlugins()
})
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <div class="text-h5 font-weight-bold">Plugin Manager</div>
      <v-spacer />
      <v-btn
        size="small"
        variant="tonal"
        color="primary"
        prepend-icon="mdi-upload"
        @click="triggerUpload"
      >
        Upload Plugin
      </v-btn>
      <input
        ref="uploadInput"
        type="file"
        accept=".cs"
        style="display: none;"
        @change="handleUpload"
      />
    </div>

    <v-tabs v-model="tab" color="primary" class="mb-4">
      <v-tab value="installed">
        Installed
        <v-chip size="x-small" class="ml-2" color="primary">{{ plugins.length }}</v-chip>
      </v-tab>
      <v-tab value="umod">Browse uMod</v-tab>
    </v-tabs>

    <v-window v-model="tab">
      <!-- Installed Plugins -->
      <v-window-item value="installed">
        <v-card>
          <v-card-text>
            <v-data-table
              :headers="installedHeaders"
              :items="plugins"
              :loading="loading"
              item-key="name"
              class="elevation-0"
              density="comfortable"
            >
              <template #item.size="{ item }">
                {{ formatSize(item.size) }}
              </template>
              <template #item.modifiedAt="{ item }">
                {{ formatDate(item.modifiedAt) }}
              </template>
              <template #item.loaded="{ item }">
                <v-chip
                  :color="item.loaded ? 'success' : 'grey'"
                  size="small"
                >
                  {{ item.loaded ? 'Loaded' : 'Unloaded' }}
                </v-chip>
              </template>
              <template #item.actions="{ item }">
                <v-btn
                  v-if="item.hasConfig"
                  icon="mdi-cog"
                  size="small"
                  variant="text"
                  color="info"
                  title="Config"
                  @click="openConfig(item)"
                />
                <v-btn
                  icon="mdi-reload"
                  size="small"
                  variant="text"
                  color="warning"
                  title="Reload"
                  @click="reloadPlugin(item)"
                />
                <v-btn
                  icon="mdi-delete"
                  size="small"
                  variant="text"
                  color="error"
                  title="Delete"
                  @click="confirmDelete(item)"
                />
              </template>
              <template #no-data>
                <div class="text-center pa-4 text-grey">No plugins installed</div>
              </template>
            </v-data-table>
          </v-card-text>
        </v-card>
      </v-window-item>

      <!-- Browse uMod -->
      <v-window-item value="umod">
        <v-card class="mb-4">
          <v-card-text>
            <v-text-field
              v-model="umodSearch"
              prepend-inner-icon="mdi-magnify"
              label="Search uMod plugins..."
              hide-details
              @keydown.enter="searchUMod"
            >
              <template #append>
                <v-btn
                  color="primary"
                  variant="flat"
                  :loading="umodLoading"
                  @click="searchUMod"
                >
                  Search
                </v-btn>
              </template>
            </v-text-field>
          </v-card-text>
        </v-card>

        <v-row v-if="umodLoading">
          <v-col cols="12" class="text-center py-8">
            <v-progress-circular indeterminate color="primary" size="36" />
          </v-col>
        </v-row>

        <v-row v-else-if="umodResults.length === 0 && umodSearch">
          <v-col cols="12">
            <div class="text-center pa-8 text-grey">No results found</div>
          </v-col>
        </v-row>

        <v-row v-else>
          <v-col
            v-for="plugin in umodResults"
            :key="plugin.name"
            cols="12"
            sm="6"
            md="4"
          >
            <v-card class="fill-height d-flex flex-column">
              <v-card-title class="text-body-1 font-weight-bold">
                {{ plugin.title }}
              </v-card-title>
              <v-card-subtitle>
                by {{ plugin.author }} &bull; v{{ plugin.version }}
              </v-card-subtitle>
              <v-card-text class="flex-grow-1">
                <div class="text-body-2 text-grey mb-2" style="display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;">
                  {{ plugin.description }}
                </div>
                <v-chip size="x-small" color="info" variant="tonal">
                  <v-icon start size="12">mdi-download</v-icon>
                  {{ plugin.downloads.toLocaleString() }}
                </v-chip>
              </v-card-text>
              <v-card-actions>
                <v-spacer />
                <v-btn
                  color="primary"
                  variant="tonal"
                  size="small"
                  prepend-icon="mdi-download"
                  :loading="installingPlugin === plugin.name"
                  @click="installUModPlugin(plugin)"
                >
                  Install
                </v-btn>
              </v-card-actions>
            </v-card>
          </v-col>
        </v-row>
      </v-window-item>
    </v-window>

    <!-- Config Editor Dialog -->
    <v-dialog v-model="configDialog" max-width="700">
      <v-card>
        <v-card-title class="d-flex align-center">
          <v-icon class="mr-2">mdi-cog</v-icon>
          {{ configPlugin?.name }} - Config
        </v-card-title>
        <v-card-text>
          <textarea
            v-model="configContent"
            spellcheck="false"
            style="
              width: 100%;
              height: 400px;
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
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="configDialog = false">Cancel</v-btn>
          <v-btn color="primary" variant="flat" :loading="configSaving" @click="saveConfig">
            Save Config
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Delete Confirmation -->
    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title>Delete Plugin</v-card-title>
        <v-card-text>
          Are you sure you want to delete <strong>{{ deleteTarget?.name }}</strong>?
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">Cancel</v-btn>
          <v-btn color="error" variant="flat" @click="executeDelete">Delete</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
