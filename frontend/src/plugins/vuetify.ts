import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

const rustDarkTheme = {
  dark: true,
  colors: {
    background: '#121212',
    surface: '#1E1E1E',
    'surface-bright': '#2A2A2A',
    'surface-variant': '#333333',
    primary: '#CD412B',
    'primary-darken-1': '#A33522',
    secondary: '#FF6B4A',
    'secondary-darken-1': '#CC5538',
    error: '#CF6679',
    info: '#4FC3F7',
    success: '#66BB6A',
    warning: '#FFA726',
    'on-background': '#E0E0E0',
    'on-surface': '#E0E0E0',
  },
}

export default createVuetify({
  components,
  directives,
  theme: {
    defaultTheme: 'rustDarkTheme',
    themes: {
      rustDarkTheme,
    },
  },
  defaults: {
    VBtn: {
      variant: 'flat',
    },
    VCard: {
      elevation: 2,
    },
    VTextField: {
      variant: 'outlined',
      density: 'comfortable',
    },
  },
})
