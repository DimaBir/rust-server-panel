<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { serverApi } from '../services/api'
import { useRoute } from 'vue-router'
import type { FileEntry } from '../types'

const route = useRoute()
const serverId = computed(() => route.params.serverId as string)

const loading = ref(true)
const saving = ref(false)
const files = ref<FileEntry[]>([])
const currentPath = ref('')
const selectedFile = ref<FileEntry | null>(null)
const fileContent = ref('')
const fileModified = ref(false)

const newFileDialog = ref(false)
const newFolderDialog = ref(false)
const deleteDialog = ref(false)
const newFileName = ref('')
const newFolderName = ref('')
const deleteTarget = ref<FileEntry | null>(null)

const uploadInput = ref<HTMLInputElement | null>(null)

interface BreadcrumbItem { title: string; path: string }

const breadcrumbs = computed<BreadcrumbItem[]>(() => {
  const parts = currentPath.value.split('/').filter(Boolean)
  const crumbs: BreadcrumbItem[] = [{ title: 'Root', path: '' }]
  let accumulated = ''
  for (const part of parts) {
    accumulated += (accumulated ? '/' : '') + part
    crumbs.push({ title: part, path: accumulated })
  }
  return crumbs
})

const sortedFiles = computed(() => {
  const dirs = files.value.filter((f) => f.isDir).sort((a, b) => a.name.localeCompare(b.name))
  const regular = files.value.filter((f) => !f.isDir).sort((a, b) => a.name.localeCompare(b.name))
  return [...dirs, ...regular]
})

async function fetchFiles(path?: string) {
  if (!serverId.value) return
  loading.value = true
  try {
    const p = path ?? currentPath.value
    const api = serverApi(serverId.value)
    const res = await api.get<FileEntry[]>('/files/list', { params: { path: p } })
    files.value = res.data ?? []
    currentPath.value = p
  } catch {
    files.value = []
  } finally {
    loading.value = false
  }
}

async function openItem(item: FileEntry) {
  if (item.isDir) {
    selectedFile.value = null
    fileContent.value = ''
    fileModified.value = false
    await fetchFiles(item.path)
  } else if (item.isText) {
    await openFile(item)
  }
}

async function openFile(item: FileEntry) {
  try {
    const api = serverApi(serverId.value)
    const res = await api.get<{ content: string }>('/files/read', { params: { path: item.path } })
    selectedFile.value = item
    fileContent.value = res.data.content
    fileModified.value = false
  } catch { /* interceptor */ }
}

async function saveFile() {
  if (!selectedFile.value) return
  saving.value = true
  try {
    const api = serverApi(serverId.value)
    await api.put('/files/write', { path: selectedFile.value.path, content: fileContent.value })
    fileModified.value = false
  } catch { /* interceptor */ }
  finally { saving.value = false }
}

function navigateTo(path: string) {
  selectedFile.value = null
  fileContent.value = ''
  fileModified.value = false
  fetchFiles(path)
}

async function createFile() {
  if (!newFileName.value.trim()) return
  const path = currentPath.value ? `${currentPath.value}/${newFileName.value.trim()}` : newFileName.value.trim()
  try {
    const api = serverApi(serverId.value)
    await api.put('/files/write', { path, content: '' })
    newFileDialog.value = false
    newFileName.value = ''
    await fetchFiles()
  } catch { /* interceptor */ }
}

async function createFolder() {
  if (!newFolderName.value.trim()) return
  const path = currentPath.value ? `${currentPath.value}/${newFolderName.value.trim()}` : newFolderName.value.trim()
  try {
    const api = serverApi(serverId.value)
    await api.post('/files/mkdir', { path })
    newFolderDialog.value = false
    newFolderName.value = ''
    await fetchFiles()
  } catch { /* interceptor */ }
}

function confirmDelete(item: FileEntry) {
  deleteTarget.value = item
  deleteDialog.value = true
}

async function executeDelete() {
  if (!deleteTarget.value) return
  try {
    const api = serverApi(serverId.value)
    await api.delete('/files/delete', { params: { path: deleteTarget.value.path } })
    if (selectedFile.value?.path === deleteTarget.value.path) {
      selectedFile.value = null
      fileContent.value = ''
      fileModified.value = false
    }
    deleteDialog.value = false
    deleteTarget.value = null
    await fetchFiles()
  } catch { /* interceptor */ }
}

function triggerUpload() { uploadInput.value?.click() }

async function handleUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  const formData = new FormData()
  formData.append('file', file)
  formData.append('path', currentPath.value)

  try {
    const api = serverApi(serverId.value)
    await api.post('/files/upload', formData, { headers: { 'Content-Type': 'multipart/form-data' } })
    await fetchFiles()
  } catch { /* interceptor */ }
  finally { input.value = '' }
}

function downloadFile(item: FileEntry) {
  const token = localStorage.getItem('jwt_token')
  const url = `/api/servers/${serverId.value}/files/download?path=${encodeURIComponent(item.path)}&token=${token}`
  window.open(url, '_blank')
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

function fileIcon(item: FileEntry): string {
  if (item.isDir) return 'mdi-folder'
  const ext = item.name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'json': return 'mdi-code-json'
    case 'cfg': case 'ini': case 'conf': return 'mdi-cog'
    case 'cs': return 'mdi-language-csharp'
    case 'txt': case 'log': return 'mdi-file-document-outline'
    default: return 'mdi-file-document'
  }
}

function fileIconColor(item: FileEntry): string {
  if (item.isDir) return 'warning'
  const ext = item.name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'json': return 'info'
    case 'cs': return 'success'
    default: return 'medium-emphasis'
  }
}

onMounted(() => { fetchFiles('') })
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">File Manager</div>
      <v-spacer />
      <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-file-plus" class="mr-2" @click="newFileDialog = true">New File</v-btn>
      <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-folder-plus" class="mr-2" @click="newFolderDialog = true">New Folder</v-btn>
      <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-upload" @click="triggerUpload">Upload</v-btn>
      <input ref="uploadInput" type="file" style="display: none;" @change="handleUpload" />
    </div>

    <div class="d-flex align-center pa-0 mb-3 text-body-2">
      <template v-for="(crumb, idx) in breadcrumbs" :key="idx">
        <span v-if="idx > 0" class="mx-1 text-medium-emphasis">/</span>
        <span style="cursor: pointer;" class="text-primary" @click="navigateTo(crumb.path)">{{ crumb.title }}</span>
      </template>
    </div>

    <v-row>
      <v-col cols="12" :md="selectedFile ? 5 : 12">
        <v-card>
          <v-card-text class="pa-0">
            <v-list density="compact" class="pa-0">
              <v-list-item v-if="currentPath" @click="navigateTo(currentPath.split('/').slice(0, -1).join('/'))">
                <template #prepend><v-icon color="medium-emphasis">mdi-arrow-up</v-icon></template>
                <v-list-item-title>..</v-list-item-title>
              </v-list-item>
              <template v-if="loading">
                <v-list-item><div class="text-center pa-4 w-100"><v-progress-circular indeterminate color="primary" size="24" /></div></v-list-item>
              </template>
              <template v-else-if="sortedFiles.length === 0">
                <v-list-item><div class="text-center pa-4 text-medium-emphasis">Empty directory</div></v-list-item>
              </template>
              <v-list-item
                v-for="item in sortedFiles"
                :key="item.path"
                :active="selectedFile?.path === item.path"
                color="primary"
                @click="openItem(item)"
              >
                <template #prepend><v-icon :color="fileIconColor(item)">{{ fileIcon(item) }}</v-icon></template>
                <v-list-item-title>{{ item.name }}</v-list-item-title>
                <v-list-item-subtitle v-if="!item.isDir">{{ formatSize(item.size) }} &bull; {{ formatDate(item.modified) }}</v-list-item-subtitle>
                <template #append>
                  <v-btn v-if="!item.isDir" icon="mdi-download" size="x-small" variant="text" color="medium-emphasis" @click.stop="downloadFile(item)" />
                  <v-btn icon="mdi-delete" size="x-small" variant="text" color="error" @click.stop="confirmDelete(item)" />
                </template>
              </v-list-item>
            </v-list>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="selectedFile" cols="12" md="7">
        <v-card class="d-flex flex-column" style="height: calc(100vh - 220px);">
          <v-card-title class="d-flex align-center py-2">
            <v-icon size="small" class="mr-2">mdi-file-document-edit</v-icon>
            {{ selectedFile.name }}
            <v-chip v-if="fileModified" size="x-small" color="warning" variant="tonal" class="ml-2">Modified</v-chip>
            <v-spacer />
            <v-btn size="small" variant="flat" color="primary" prepend-icon="mdi-content-save" :loading="saving" :disabled="!fileModified" @click="saveFile">Save</v-btn>
            <v-btn size="small" variant="text" color="medium-emphasis" icon="mdi-close" class="ml-1" @click="selectedFile = null; fileContent = ''; fileModified = false" />
          </v-card-title>
          <v-divider />
          <v-card-text class="flex-grow-1 pa-0" style="overflow: hidden;">
            <textarea
              v-model="fileContent"
              spellcheck="false"
              style="width: 100%; height: 100%; background: #0a0a0b; color: #e2e8f0; border: none; outline: none; padding: 12px; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 13px; line-height: 1.5; resize: none; tab-size: 2;"
              @input="fileModified = true"
            />
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <v-dialog v-model="newFileDialog" max-width="450">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">New File</v-card-title>
        <v-card-text><v-text-field v-model="newFileName" label="File name" placeholder="example.cfg" hide-details autofocus @keydown.enter="createFile" /></v-card-text>
        <v-card-actions><v-spacer /><v-btn variant="text" @click="newFileDialog = false">Cancel</v-btn><v-btn color="primary" variant="flat" :disabled="!newFileName.trim()" @click="createFile">Create</v-btn></v-card-actions>
      </v-card>
    </v-dialog>
    <v-dialog v-model="newFolderDialog" max-width="450">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">New Folder</v-card-title>
        <v-card-text><v-text-field v-model="newFolderName" label="Folder name" hide-details autofocus @keydown.enter="createFolder" /></v-card-text>
        <v-card-actions><v-spacer /><v-btn variant="text" @click="newFolderDialog = false">Cancel</v-btn><v-btn color="primary" variant="flat" :disabled="!newFolderName.trim()" @click="createFolder">Create</v-btn></v-card-actions>
      </v-card>
    </v-dialog>
    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Delete {{ deleteTarget?.isDir ? 'Folder' : 'File' }}</v-card-title>
        <v-card-text>Are you sure you want to delete <strong>{{ deleteTarget?.name }}</strong>?</v-card-text>
        <v-card-actions><v-spacer /><v-btn variant="text" @click="deleteDialog = false">Cancel</v-btn><v-btn color="error" variant="flat" @click="executeDelete">Delete</v-btn></v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
