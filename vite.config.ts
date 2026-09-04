/// <reference types="vitest" />
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const pkg = JSON.parse(
  readFileSync(fileURLToPath(new URL('./package.json', import.meta.url)), 'utf-8'),
) as { version: string }

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: 'splash-version',
      transformIndexHtml: (html) => html.replaceAll('__APP_VERSION__', pkg.version),
    },
  ],
  clearScreen: false,
  resolve: {
    alias: {
      '@data': fileURLToPath(new URL('./data', import.meta.url)),
    },
  },
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  build: {
    sourcemap: true,
    assetsInlineLimit: 0,
  },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 5174 }
      : undefined,
    watch: { ignored: ['**/engine/**'] },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./frontend/test/setup.ts'],
    include: ['{frontend,data}/**/*.{test,spec}.{ts,tsx}'],
    css: false,
  },
})
