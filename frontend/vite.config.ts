import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Dev server proxies API + uploaded images to the Rust backend (port 8787),
// so the app uses same-origin relative URLs in both dev and prod.
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      '/api': 'http://localhost:8787',
      '/uploads': 'http://localhost:8787',
    },
  },
})
