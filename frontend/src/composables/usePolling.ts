import { ref, onUnmounted } from 'vue'

export function usePolling<T>(
  fetchFn: () => Promise<T>,
  intervalMs: number
) {
  const data = ref<T | null>(null) as { value: T | null }
  const loading = ref(false)
  const error = ref<string | null>(null)

  let timer: ReturnType<typeof setInterval> | null = null
  let active = false

  async function poll() {
    try {
      loading.value = true
      data.value = await fetchFn()
      error.value = null
    } catch (err: unknown) {
      if (err instanceof Error) {
        error.value = err.message
      } else {
        error.value = 'Polling error'
      }
    } finally {
      loading.value = false
    }
  }

  function start() {
    if (active) return
    active = true
    poll()
    timer = setInterval(poll, intervalMs)
  }

  function stop() {
    active = false
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  function restart() {
    stop()
    start()
  }

  onUnmounted(() => {
    stop()
  })

  return {
    data,
    loading,
    error,
    start,
    stop,
    restart,
    poll,
  }
}
