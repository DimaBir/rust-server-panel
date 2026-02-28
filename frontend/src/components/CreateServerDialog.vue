<script setup lang="ts">
import { ref, computed } from 'vue'
import { useServerStore } from '../stores/server'
import type { CreateServerRequest } from '../types'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  created: [id: string]
}>()

const serverStore = useServerStore()

const step = ref(1)
const creating = ref(false)

const serverType = ref<'vanilla' | 'modded'>('vanilla')
const name = ref('')
const hostname = ref('')
const maxPlayers = ref(100)
const worldSize = ref(4000)
const seed = ref<number | undefined>(undefined)

const worldSizes = [
  { title: 'Small (2000)', value: 2000 },
  { title: 'Medium (3000)', value: 3000 },
  { title: 'Default (4000)', value: 4000 },
  { title: 'Large (6000)', value: 6000 },
]

const isOpen = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

function selectType(type: 'vanilla' | 'modded') {
  serverType.value = type
  step.value = 2
}

function back() {
  step.value = 1
}

function reset() {
  step.value = 1
  name.value = ''
  hostname.value = ''
  maxPlayers.value = 100
  worldSize.value = 4000
  seed.value = undefined
  serverType.value = 'vanilla'
  creating.value = false
}

async function create() {
  if (!name.value.trim()) return
  creating.value = true
  try {
    const req: CreateServerRequest = {
      name: name.value.trim(),
      serverType: serverType.value,
      maxPlayers: maxPlayers.value,
      worldSize: worldSize.value,
    }
    if (hostname.value.trim()) req.hostname = hostname.value.trim()
    if (seed.value != null && seed.value > 0) req.seed = seed.value

    const result = await serverStore.createServer(req)
    if (result) {
      emit('created', result.id)
      isOpen.value = false
      reset()
    }
  } finally {
    creating.value = false
  }
}
</script>

<template>
  <v-dialog v-model="isOpen" max-width="600" @after-leave="reset">
    <v-card>
      <v-card-title class="text-h6 font-weight-medium d-flex align-center">
        <span>Create New Server</span>
        <v-spacer />
        <v-chip size="small" variant="tonal" color="primary">Step {{ step }}/2</v-chip>
      </v-card-title>

      <v-card-text>
        <!-- Step 1: Choose Type -->
        <template v-if="step === 1">
          <div class="text-body-2 text-medium-emphasis mb-4">Choose your server type:</div>
          <v-row>
            <v-col cols="6">
              <v-card
                class="pa-6 text-center"
                :style="{ cursor: 'pointer', border: serverType === 'vanilla' ? '2px solid rgb(var(--v-theme-primary))' : '1px solid rgba(255,255,255,0.1)' }"
                @click="selectType('vanilla')"
              >
                <v-icon size="48" color="info" class="mb-3">mdi-sword-cross</v-icon>
                <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Vanilla</div>
                <div class="text-caption text-medium-emphasis mt-1">Pure Rust experience</div>
              </v-card>
            </v-col>
            <v-col cols="6">
              <v-card
                class="pa-6 text-center"
                :style="{ cursor: 'pointer', border: serverType === 'modded' ? '2px solid rgb(var(--v-theme-primary))' : '1px solid rgba(255,255,255,0.1)' }"
                @click="selectType('modded')"
              >
                <v-icon size="48" color="purple" class="mb-3">mdi-puzzle</v-icon>
                <div class="text-h6 font-weight-medium" style="color: #e2e8f0;">Modded</div>
                <div class="text-caption text-medium-emphasis mt-1">With Oxide/uMod framework</div>
              </v-card>
            </v-col>
          </v-row>
        </template>

        <!-- Step 2: Configure -->
        <template v-if="step === 2">
          <v-text-field
            v-model="name"
            label="Server Name"
            placeholder="My Rust Server"
            class="mb-3"
            hide-details
            autofocus
          />
          <v-text-field
            v-model="hostname"
            label="In-Game Hostname (optional)"
            placeholder="Leave blank to use server name"
            class="mb-3"
            hide-details
          />
          <v-row>
            <v-col cols="6">
              <v-text-field
                v-model.number="maxPlayers"
                label="Max Players"
                type="number"
                hide-details
              />
            </v-col>
            <v-col cols="6">
              <v-select
                v-model="worldSize"
                :items="worldSizes"
                label="World Size"
                hide-details
              />
            </v-col>
          </v-row>
          <v-text-field
            v-model.number="seed"
            label="Map Seed (optional, random if blank)"
            type="number"
            class="mt-3"
            hide-details
          />
        </template>
      </v-card-text>

      <v-card-actions>
        <v-btn v-if="step === 2" variant="text" @click="back">Back</v-btn>
        <v-spacer />
        <v-btn variant="text" @click="isOpen = false">Cancel</v-btn>
        <v-btn
          v-if="step === 2"
          color="primary"
          variant="flat"
          :loading="creating"
          :disabled="!name.trim()"
          @click="create"
        >
          Create Server
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
