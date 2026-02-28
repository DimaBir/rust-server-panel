import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('jwt_token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('jwt_token')
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)

/** Create a scoped API helper for a specific server */
export function serverApi(serverId: string) {
  return {
    get: <T = unknown>(path: string, config?: Parameters<typeof api.get>[1]) =>
      api.get<T>(`/servers/${serverId}${path}`, config),
    post: <T = unknown>(path: string, data?: unknown, config?: Parameters<typeof api.post>[2]) =>
      api.post<T>(`/servers/${serverId}${path}`, data, config),
    put: <T = unknown>(path: string, data?: unknown, config?: Parameters<typeof api.put>[2]) =>
      api.put<T>(`/servers/${serverId}${path}`, data, config),
    delete: <T = unknown>(path: string, config?: Parameters<typeof api.delete>[1]) =>
      api.delete<T>(`/servers/${serverId}${path}`, config),
  }
}

export default api
