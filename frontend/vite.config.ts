import { defineConfig, type Plugin } from 'vite';
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const PYODIDE_FILES = ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json'];

function pyodideFiles(): Plugin {
  let outDir = resolve(__dirname, 'dist');
  return {
    name: 'formsurface-pyodide-files',
    apply: 'build',
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    closeBundle() {
      const to = resolve(outDir, 'pyodide');
      mkdirSync(to, { recursive: true });
      for (const file of PYODIDE_FILES) copyFileSync(resolve(__dirname, 'node_modules/pyodide', file), resolve(to, file));
    },
  };
}

export default defineConfig({
  build: { target: 'es2022' },
  plugins: [pyodideFiles()],
  optimizeDeps: { exclude: ['pyodide'] },
  server: {
    host: false,
    port: 5176,
    strictPort: true,
    fs: { allow: ['.', '../mcp/formsurface_api', '../mcp/server.py'] },
    proxy: { '/api': { target: 'http://127.0.0.1:8766', ws: true } },
  },
  preview: {
    host: false,
    port: 5177,
    strictPort: true,
    proxy: { '/api': { target: 'http://127.0.0.1:8766' } },
  },
});
