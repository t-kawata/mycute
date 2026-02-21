import type { RouteRecordRaw } from 'vue-router';

export const URL = {
  HOME: '/app',
  LOGIN: '/login',
  CAPTURE: '/capture',
  MERII_CAPTURE: '/mcapture',
  MUSIC: '/music',
  SETTINGS: '/settings-app',
}

// 先頭のスラッシュ除去
const toRelativePath = (path: string) => path.replace(/^\//, '');

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
      { path: toRelativePath(URL.MUSIC), component: () => import('src/apps/MusicApp.vue') },
      { path: toRelativePath(URL.SETTINGS), component: () => import('src/apps/SettingsApp.vue') },
    ],
  },
  {
    path: URL.LOGIN,
    component: () => import('layouts/LoginLayout.vue'),
    children: [{ path: '', component: () => import('pages/LoginPage.vue') }],
  },
  {
    path: URL.CAPTURE,
    component: () => import('layouts/CaptureLayout.vue'),
    children: [{ path: '', component: () => import('pages/CapturePage.vue') }],
  },
  {
    path: URL.MERII_CAPTURE,
    component: () => import('layouts/CaptureLayout.vue'),
    children: [{ path: '', component: () => import('pages/MeriiCapturePage.vue') }],
  },

  // Always leave this as last one,
  // but you can also remove it
  {
    path: '/:catchAll(.*)*',
    component: () => import('pages/ErrorNotFound.vue'),
  },
];

export default routes;
