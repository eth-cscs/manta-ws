import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'login',
      component: () => import('../views/LoginView.vue')
    },
    {
      path: '/about',
      name: 'about',
      // route level code-splitting
      // this generates a separate chunk (About.[hash].js) for this route
      // which is lazy-loaded when the route is visited.
      component: () => import('../views/AboutView.vue')
    },
    {
      path: '/hsm',
      name: 'hsmgroupsummary',

      component: () => import('../views/HsmGroupSummaryView.vue')
    },
    {
      path: '/hsm/:hsm',
      name: 'hsmgroupdetails',

      component: () => import('../views/HsmGroupDetailsView.vue')
    },
    {
      path: '/hsm/:hsm/hardware',
      name: 'hsmgrouphardware',

      component: () => import('../views/HsmGroupHardwareView.vue')
    },
    {
      path: '/console/:xname',
      name: 'console',
      component: () => import('../views/ConsoleView.vue')
    },
    {
      path: '/cfssessions',
      name: 'listcfssessions',
      component: () => import('../views/ListCfsSessionsView.vue')
    },
    {
      path: '/cfssession/:cfssession/logs',
      name: 'cfssessionlogs',
      component: () => import('../views/CfsSessionLogsView.vue')
    },
    {
      path: '/cfssession/:cfssession/logs',
      name: 'cfssessionlogs',
      component: () => import('../views/CfsSessionLogsView.vue')
    },
  ]
})

export default router
