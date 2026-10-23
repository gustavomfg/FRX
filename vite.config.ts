import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// O Vite serve a interface para o Tauri em desenvolvimento (devUrl) e gera
// os arquivos estáticos que o Tauri embute no binário em produção (frontendDist).
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': new URL('./src', import.meta.url).pathname } },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    // O Rust é observado pelo próprio Tauri.
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  // Caminhos relativos: os assets são carregados do próprio pacote, sem servidor.
  base: './',
  build: {
    // WebKitGTK 2.4x+ entende ES2022 sem transpilar.
    target: 'es2022',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    minify: !process.env.TAURI_ENV_DEBUG,
    chunkSizeWarningLimit: 600,
  },
})
