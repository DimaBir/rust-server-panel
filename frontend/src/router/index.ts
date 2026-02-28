import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('../views/LoginPage.vue'),
      meta: { requiresAuth: false },
    },
    {
      path: '/',
      component: () => import('../layouts/MainLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'Dashboard',
          component: () => import('../views/DashboardPage.vue'),
        },
        {
          path: 'console',
          name: 'Console',
          component: () => import('../views/ConsolePage.vue'),
        },
        {
          path: 'players',
          name: 'Players',
          component: () => import('../views/PlayersPage.vue'),
        },
        {
          path: 'files',
          name: 'Files',
          component: () => import('../views/FilesPage.vue'),
        },
        {
          path: 'plugins',
          name: 'Plugins',
          component: () => import('../views/PluginsPage.vue'),
        },
        {
          path: 'config',
          name: 'Config',
          component: () => import('../views/ConfigPage.vue'),
        },
        {
          path: 'logs',
          name: 'Logs',
          component: () => import('../views/LogsPage.vue'),
        },
        {
          path: 'schedule',
          name: 'Schedule',
          component: () => import('../views/SchedulePage.vue'),
        },
      ],
    },
  ],
})

router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('jwt_token')
  if (to.meta.requiresAuth !== false && !token) {
    next('/login')
  } else if (to.path === '/login' && token) {
    next('/')
  } else {
    next()
  }
})

export default router
