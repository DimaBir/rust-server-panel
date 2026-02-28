import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../services/api'
import type { LoginRequest, LoginResponse } from '../types'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('jwt_token'))
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value)

  async function login(credentials: LoginRequest): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      const response = await api.post<LoginResponse>('/auth/login', credentials)
      token.value = response.data.token
      localStorage.setItem('jwt_token', response.data.token)
      return true
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'response' in err) {
        const axiosErr = err as { response?: { data?: { message?: string } } }
        error.value = axiosErr.response?.data?.message || 'Login failed'
      } else {
        error.value = 'Network error'
      }
      return false
    } finally {
      loading.value = false
    }
  }

  function logout() {
    token.value = null
    localStorage.removeItem('jwt_token')
  }

  return {
    token,
    loading,
    error,
    isAuthenticated,
    login,
    logout,
  }
})
