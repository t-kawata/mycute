import type { RouteRecordRaw } from 'vue-router';

export const URL = {
  HOME: '/app',
  LOGIN: '/login',
  LLM: '/llm',
  SETTINGS: '/settings-app',
}

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('layouts/SplashLayout.vue'),
    children: [{ path: '', component: () => import('pages/SplashPage.vue') }],
  },
  {
    path: URL.HOME,
    component: () => import('layouts/MainLayout.vue'),
    children: [
      { path: '', component: () => import('src/apps/HarunohiApp.vue') },
      { path: URL.LLM, component: () => import('src/apps/LlmApp.vue') },
      { path: URL.SETTINGS, component: () => import('src/apps/SettingsApp.vue') },
    ],
  },
  {
    path: URL.LOGIN,
    component: () => import('layouts/LoginLayout.vue'),
    children: [{ path: '', component: () => import('pages/LoginPage.vue') }],
  },

  // Always leave this as last one,
  // but you can also remove it
  {
    path: '/:catchAll(.*)*',
    component: () => import('pages/ErrorNotFound.vue'),
  },
];

export default routes;
