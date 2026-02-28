<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '../services/api'
import type { ScheduledJob } from '../types'

const loading = ref(true)
const jobs = ref<ScheduledJob[]>([])

const editDialog = ref(false)
const deleteDialog = ref(false)
const editingJob = ref<Partial<ScheduledJob>>({})
const isNewJob = ref(false)
const saving = ref(false)
const deleteTarget = ref<ScheduledJob | null>(null)

const headers = [
  { title: 'Name', key: 'name' },
  { title: 'Schedule', key: 'schedule' },
  { title: 'Action', key: 'action' },
  { title: 'Status', key: 'enabled' },
  { title: 'Actions', key: 'actions', sortable: false },
]

const actionTypes = [
  { title: 'Console Command', value: 'command' },
  { title: 'Restart Server', value: 'restart' },
  { title: 'Save World', value: 'save' },
  { title: 'Broadcast Message', value: 'broadcast' },
  { title: 'Wipe Map', value: 'wipe_map' },
]

const schedulePresets = [
  { label: 'Every Hour', value: '0 * * * *' },
  { label: 'Every 6 Hours', value: '0 */6 * * *' },
  { label: 'Daily 4am', value: '0 4 * * *' },
  { label: 'Daily 12pm', value: '0 12 * * *' },
  { label: 'Weekly Monday 4am', value: '0 4 * * 1' },
  { label: 'Every 30 min', value: '*/30 * * * *' },
]

async function fetchJobs() {
  loading.value = true
  try {
    const res = await api.get<{ jobs: ScheduledJob[] }>('/schedule')
    jobs.value = res.data.jobs ?? []
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
    action: 'command',
    params: {},
    enabled: true,
  }
  isNewJob.value = true
  editDialog.value = true
}

function openEditJob(job: ScheduledJob) {
  editingJob.value = { ...job, params: { ...job.params } }
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
  } catch {
    // Error handled by interceptor
  } finally {
    saving.value = false
  }
}

async function toggleJob(job: ScheduledJob) {
  try {
    await api.post(`/schedule/${job.id}/toggle`)
    job.enabled = !job.enabled
  } catch {
    // Error handled by interceptor
  }
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
  } catch {
    // Error handled by interceptor
  }
}

function applyPreset(preset: string) {
  editingJob.value.schedule = preset
}

function actionLabel(value: string): string {
  return actionTypes.find((a) => a.value === value)?.title ?? value
}

function getParamValue(key: string): string {
  return editingJob.value.params?.[key] ?? ''
}

function setParamValue(key: string, value: string) {
  if (!editingJob.value.params) {
    editingJob.value.params = {}
  }
  editingJob.value.params[key] = value
}

onMounted(() => {
  fetchJobs()
})
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <div class="text-h5 font-weight-bold">Scheduled Tasks</div>
      <v-spacer />
      <v-btn
        color="primary"
        prepend-icon="mdi-plus"
        size="small"
        @click="openNewJob"
      >
        Add Task
      </v-btn>
    </div>

    <v-card>
      <v-card-text>
        <v-data-table
          :headers="headers"
          :items="jobs"
          :loading="loading"
          item-key="id"
          class="elevation-0"
          density="comfortable"
        >
          <template #item.action="{ item }">
            <v-chip size="small" variant="tonal" color="info">
              {{ actionLabel(item.action) }}
            </v-chip>
          </template>
          <template #item.enabled="{ item }">
            <v-switch
              :model-value="item.enabled"
              color="success"
              density="compact"
              hide-details
              @update:model-value="toggleJob(item)"
            />
          </template>
          <template #item.actions="{ item }">
            <v-btn
              icon="mdi-pencil"
              size="small"
              variant="text"
              color="info"
              title="Edit"
              @click="openEditJob(item)"
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
            <div class="text-center pa-8 text-grey">
              <v-icon size="48" color="grey" class="mb-2">mdi-clock-outline</v-icon>
              <div>No scheduled tasks</div>
              <v-btn
                color="primary"
                variant="tonal"
                size="small"
                class="mt-3"
                prepend-icon="mdi-plus"
                @click="openNewJob"
              >
                Create First Task
              </v-btn>
            </div>
          </template>
        </v-data-table>
      </v-card-text>
    </v-card>

    <!-- Add/Edit Dialog -->
    <v-dialog v-model="editDialog" max-width="600">
      <v-card>
        <v-card-title>
          {{ isNewJob ? 'New Scheduled Task' : 'Edit Scheduled Task' }}
        </v-card-title>
        <v-card-text>
          <v-text-field
            v-model="editingJob.name"
            label="Task Name"
            placeholder="Daily restart"
            class="mb-3"
            hide-details
          />

          <v-text-field
            v-model="editingJob.schedule"
            label="Cron Schedule"
            placeholder="0 4 * * *"
            hint="minute hour day month weekday"
            persistent-hint
            class="mb-2"
          />

          <div class="d-flex flex-wrap ga-1 mb-4">
            <v-btn
              v-for="preset in schedulePresets"
              :key="preset.value"
              size="x-small"
              variant="tonal"
              color="primary"
              @click="applyPreset(preset.value)"
            >
              {{ preset.label }}
            </v-btn>
          </div>

          <v-select
            v-model="editingJob.action"
            :items="actionTypes"
            label="Action Type"
            class="mb-3"
            hide-details
          />

          <!-- Dynamic params based on action type -->
          <template v-if="editingJob.action === 'command'">
            <v-text-field
              :model-value="getParamValue('command')"
              label="Console Command"
              placeholder="say Hello World!"
              hide-details
              @update:model-value="setParamValue('command', $event)"
            />
          </template>

          <template v-if="editingJob.action === 'broadcast'">
            <v-text-field
              :model-value="getParamValue('message')"
              label="Broadcast Message"
              placeholder="Server restarting in 5 minutes..."
              hide-details
              @update:model-value="setParamValue('message', $event)"
            />
          </template>

          <template v-if="editingJob.action === 'restart'">
            <v-text-field
              :model-value="getParamValue('delay')"
              label="Delay (seconds)"
              placeholder="0"
              type="number"
              hide-details
              @update:model-value="setParamValue('delay', $event)"
            />
          </template>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="editDialog = false">Cancel</v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :loading="saving"
            :disabled="!editingJob.name || !editingJob.schedule || !editingJob.action"
            @click="saveJob"
          >
            {{ isNewJob ? 'Create' : 'Save' }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Delete Confirmation -->
    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title>Delete Scheduled Task</v-card-title>
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
