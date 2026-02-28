<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { serverApi } from '../services/api'
import { useRoute } from 'vue-router'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const loading = ref(true)
const saving = ref(false)
const serverCfg = ref('')
const cfgModified = ref(false)

async function fetchServerCfg() {
  if (!serverId.value) return
  loading.value = true
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ content: string }>('/files/read', {
      params: { path: 'serverfiles/server/rustserver/cfg/server.cfg' },
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
    const api = serverApi(serverId.value)
    await api.put('/files/write', {
      path: 'serverfiles/server/rustserver/cfg/server.cfg',
      content: serverCfg.value,
    })
    cfgModified.value = false
  } catch { /* interceptor */ }
  finally { saving.value = false }
}

onMounted(() => { fetchServerCfg() })
</script>

<template>
  <div>
    <div class="text-h6 font-weight-medium mb-4" style="color: #e2e8f0;">Server Configuration</div>

    <v-card>
      <v-card-title class="d-flex align-center py-3">
        <v-icon size="small" class="mr-2">mdi-file-document-edit</v-icon>
        <span class="text-body-1 font-weight-medium">server.cfg</span>
        <v-chip v-if="cfgModified" size="x-small" color="warning" variant="tonal" class="ml-2">Modified</v-chip>
        <v-spacer />
        <v-btn color="primary" variant="flat" size="small" prepend-icon="mdi-content-save" :loading="saving" :disabled="!cfgModified" @click="saveServerCfg">Save</v-btn>
      </v-card-title>
      <v-card-text>
        <div v-if="loading" class="text-center pa-8">
          <v-progress-circular indeterminate color="primary" size="36" />
        </div>
        <textarea
          v-else
          v-model="serverCfg"
          spellcheck="false"
          style="width: 100%; height: calc(100vh - 280px); min-height: 400px; background: #0a0a0b; color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; outline: none; padding: 12px; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 13px; line-height: 1.5; resize: vertical; tab-size: 2;"
          @input="cfgModified = true"
        />
      </v-card-text>
    </v-card>
  </div>
</template>
