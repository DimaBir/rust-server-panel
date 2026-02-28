import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

const panelDarkTheme = {
  dark: true,
  colors: {
    background: '#0a0a0b',
    surface: '#111113',
    'surface-bright': '#1a1a1f',
    'surface-variant': '#252529',
    primary: '#3b82f6',
    'primary-darken-1': '#2563eb',
    secondary: '#6366f1',
    'secondary-darken-1': '#4f46e5',
    error: '#ef4444',
    info: '#3b82f6',
    success: '#10b981',
    warning: '#f59e0b',
    'on-background': '#e2e8f0',
    'on-surface': '#e2e8f0',
  },
}

export default createVuetify({
  components,
  directives,
  theme: {
    defaultTheme: 'panelDarkTheme',
    themes: {
      panelDarkTheme,
    },
  },
  defaults: {
    VBtn: {
      variant: 'flat',
    },
    VCard: {
      elevation: 0,
      border: true,
      rounded: 'lg',
    },
    VTextField: {
      variant: 'outlined',
      density: 'comfortable',
    },
  },
})
