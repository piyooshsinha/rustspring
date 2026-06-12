import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// In dev, /api requests proxy to the Rust backend — run both `cargo run`
// and `npm run dev`, then develop against http://localhost:5173.
// In prod, `npm run build` emits dist/, which the backend serves itself.
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://localhost:8080',
      '/actuator': 'http://localhost:8080',
    },
  },
})
