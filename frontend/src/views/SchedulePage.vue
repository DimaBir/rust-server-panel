<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import api from '../services/api'
import { useServerStore } from '../stores/server'
import type { ScheduledJob } from '../types'

const serverStore = useServerStore()
const loading = ref(true)
const jobs = ref<ScheduledJob[]>([])

const editDialog = ref(false)
const deleteDialog = ref(false)
const editingJob = ref<Partial<ScheduledJob>>({})
const isNewJob = ref(false)
const saving = ref(false)
const deleteTarget = ref<ScheduledJob | null>(null)

const activeServerId = computed(() => serverStore.activeServerId ?? '')

const headers = [
  { title: 'Name', key: 'name' },
  { title: 'Schedule', key: 'schedule' },
  { title: 'Type', key: 'jobType' },
  { title: 'Server', key: 'serverId' },
  { title: 'Status', key: 'enabled' },
  { title: 'Actions', key: 'actions', sortable: false },
]

const jobTypes = [
  { title: 'RCON Command', value: 'rcon_command' },
  { title: 'Restart Server', value: 'restart' },
  { title: 'Update Server', value: 'update' },
  { title: 'Backup', value: 'backup' },
  { title: 'Save World', value: 'save' },
  { title: 'Announce', value: 'announce' },
  { title: 'Wipe Map', value: 'wipe_map' },
  { title: 'Wipe Full', value: 'wipe_full' },
]

const schedulePresets = [
  { label: 'Every Hour', value: '0 * * * *' },
  { label: 'Every 6 Hours', value: '0 */6 * * *' },
  { label: 'Daily 4am', value: '0 4 * * *' },
  { label: 'Daily 12pm', value: '0 12 * * *' },
  { label: 'Weekly Monday 4am', value: '0 4 * * 1' },
  { label: 'Every 30 min', value: '*/30 * * * *' },
]

// Filter to show jobs for active server
const filteredJobs = computed(() => {
  if (!activeServerId.value) return jobs.value
  return jobs.value.filter((j) => j.serverId === activeServerId.value)
})

async function fetchJobs() {
  loading.value = true
  try {
    const res = await api.get<ScheduledJob[]>('/schedule')
    jobs.value = res.data ?? []
  } catch {
    jobs.value = []
  } finally {
    loading.value = false
  }
}

function openNewJob() {
  editingJob.value = {
    name: '',
    schedule: '',
    jobType: 'rcon_command',
    payload: '',
    enabled: true,
    serverId: activeServerId.value,
  }
  isNewJob.value = true
  editDialog.value = true
}

function openEditJob(job: ScheduledJob) {
  editingJob.value = { ...job }
  isNewJob.value = false
  editDialog.value = true
}

async function saveJob() {
  saving.value = true
  try {
    if (isNewJob.value) {
      await api.post('/schedule', editingJob.value)
    } else {
      await api.put(`/schedule/${editingJob.value.id}`, editingJob.value)
    }
    editDialog.value = false
    await fetchJobs()
  } catch { /* interceptor */ }
  finally { saving.value = false }
}

async function toggleJob(job: ScheduledJob) {
  try {
    await api.post(`/schedule/${job.id}/toggle`)
    job.enabled = !job.enabled
  } catch { /* interceptor */ }
}

function confirmDelete(job: ScheduledJob) {
  deleteTarget.value = job
  deleteDialog.value = true
}

async function executeDelete() {
  if (!deleteTarget.value) return
  try {
    await api.delete(`/schedule/${deleteTarget.value.id}`)
    deleteDialog.value = false
    deleteTarget.value = null
    await fetchJobs()
  } catch { /* interceptor */ }
}

function applyPreset(preset: string) {
  editingJob.value.schedule = preset
}

function jobTypeLabel(value: string): string {
  return jobTypes.find((a) => a.value === value)?.title ?? value
}

function serverName(id: string): string {
  return serverStore.servers.find((s) => s.id === id)?.name ?? id
}

watch(() => serverStore.activeServerId, () => { fetchJobs() })

onMounted(() => { fetchJobs() })
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Scheduled Tasks</div>
      <v-spacer />
      <v-btn color="primary" prepend-icon="mdi-plus" size="small" @click="openNewJob">Add Task</v-btn>
    </div>

    <v-card>
      <v-card-text>
        <v-data-table :headers="headers" :items="filteredJobs" :loading="loading" item-key="id" class="elevation-0" density="comfortable">
          <template #item.jobType="{ item }">
            <v-chip size="small" variant="tonal" color="info">{{ jobTypeLabel(item.jobType) }}</v-chip>
          </template>
          <template #item.serverId="{ item }">
            <span class="text-medium-emphasis">{{ serverName(item.serverId) }}</span>
          </template>
          <template #item.enabled="{ item }">
            <v-switch :model-value="item.enabled" color="success" density="compact" hide-details @update:model-value="toggleJob(item)" />
          </template>
          <template #item.actions="{ item }">
            <v-btn icon="mdi-pencil" size="small" variant="text" color="medium-emphasis" @click="openEditJob(item)" />
            <v-btn icon="mdi-delete" size="small" variant="text" color="error" @click="confirmDelete(item)" />
          </template>
          <template #no-data>
            <div class="text-center pa-8 text-medium-emphasis">
              <v-icon size="48" class="mb-2">mdi-clock-outline</v-icon>
              <div>No scheduled tasks</div>
              <v-btn color="primary" variant="tonal" size="small" class="mt-3" prepend-icon="mdi-plus" @click="openNewJob">Create First Task</v-btn>
            </div>
          </template>
        </v-data-table>
      </v-card-text>
    </v-card>

    <v-dialog v-model="editDialog" max-width="600">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">{{ isNewJob ? 'New Scheduled Task' : 'Edit Scheduled Task' }}</v-card-title>
        <v-card-text>
          <v-text-field v-model="editingJob.name" label="Task Name" placeholder="Daily restart" class="mb-3" hide-details />
          <v-text-field v-model="editingJob.schedule" label="Cron Schedule" placeholder="0 4 * * *" hint="minute hour day month weekday" persistent-hint class="mb-2" />

          <div class="d-flex flex-wrap ga-1 mb-4">
            <v-btn v-for="preset in schedulePresets" :key="preset.value" size="x-small" variant="tonal" color="primary" @click="applyPreset(preset.value)">{{ preset.label }}</v-btn>
          </div>

          <v-select v-model="editingJob.jobType" :items="jobTypes" label="Job Type" class="mb-3" hide-details />

          <template v-if="editingJob.jobType === 'rcon_command'">
            <v-text-field v-model="editingJob.payload" label="Console Command" placeholder="say Hello World!" hide-details />
          </template>
          <template v-if="editingJob.jobType === 'announce'">
            <v-text-field v-model="editingJob.payload" label="Broadcast Message" placeholder="Server restarting in 5 minutes..." hide-details />
          </template>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="editDialog = false">Cancel</v-btn>
          <v-btn color="primary" variant="flat" :loading="saving" :disabled="!editingJob.name || !editingJob.schedule || !editingJob.jobType" @click="saveJob">{{ isNewJob ? 'Create' : 'Save' }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title class="text-h6 font-weight-medium">Delete Scheduled Task</v-card-title>
        <v-card-text>Are you sure you want to delete <strong>{{ deleteTarget?.name }}</strong>?</v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">Cancel</v-btn>
          <v-btn color="error" variant="flat" @click="executeDelete">Delete</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
