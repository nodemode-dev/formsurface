import { defineConfig } from 'vite';

export default defineConfig({
  build: { target: 'es2022' },
  server: {
    host: false,
    port: 5176,
    strictPort: true,
    proxy: { '/api': { target: 'http://127.0.0.1:8766', ws: true } },
  },
  preview: {
    host: false,
    port: 5177,
    strictPort: true,
    proxy: { '/api': { target: 'http://127.0.0.1:8766' } },
  },
});
