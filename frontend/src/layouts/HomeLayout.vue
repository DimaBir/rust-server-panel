<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

function logout() {
  authStore.logout()
  router.push('/login')
}
</script>

<template>
  <v-app-bar flat class="bg-surface" border="b" density="compact" style="border-color: rgba(255,255,255,0.06) !important;">
    <v-icon size="28" color="primary" class="ml-4 mr-2">mdi-server</v-icon>
    <span class="text-h6 font-weight-medium" style="color: #e2e8f0; letter-spacing: -0.3px;">Rust Panel</span>
    <v-spacer />
    <v-btn variant="text" color="medium-emphasis" prepend-icon="mdi-logout" size="small" @click="logout">
      Logout
    </v-btn>
  </v-app-bar>

  <v-main class="bg-background">
    <v-container fluid class="pa-6">
      <router-view />
    </v-container>
  </v-main>
</template>
