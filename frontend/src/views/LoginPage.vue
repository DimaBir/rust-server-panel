<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const username = ref('')
const password = ref('')
const showPassword = ref(false)

async function handleLogin() {
  const success = await authStore.login({
    username: username.value,
    password: password.value,
  })
  if (success) {
    router.push('/')
  }
}
</script>

<template>
  <v-container fluid class="fill-height" style="background: #0a0a0b;">
    <v-row justify="center" align="center" class="fill-height">
      <v-col cols="12" sm="8" md="5" lg="4" xl="3">
        <v-card class="pa-6" style="background: #111113; border: 1px solid rgba(255,255,255,0.08);">
          <div class="text-center mb-6">
            <v-icon size="64" color="primary" class="mb-3">mdi-server</v-icon>
            <div class="text-h4 font-weight-bold" style="color: #e2e8f0;">
              RUST PANEL
            </div>
            <div class="text-body-2 mt-1" style="color: #94a3b8;">Server Control Panel</div>
          </div>

          <v-alert v-if="authStore.error" type="error" variant="tonal" density="compact" class="mb-4">
            {{ authStore.error }}
          </v-alert>

          <v-form @submit.prevent="handleLogin">
            <v-text-field v-model="username" label="Username" prepend-inner-icon="mdi-account" autocomplete="username" class="mb-3" hide-details />
            <v-text-field
              v-model="password"
              :type="showPassword ? 'text' : 'password'"
              label="Password"
              prepend-inner-icon="mdi-lock"
              :append-inner-icon="showPassword ? 'mdi-eye-off' : 'mdi-eye'"
              autocomplete="current-password"
              class="mb-5"
              hide-details
              @click:append-inner="showPassword = !showPassword"
            />
            <v-btn type="submit" block size="large" color="primary" :loading="authStore.loading" :disabled="!username || !password">Login</v-btn>
          </v-form>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>
