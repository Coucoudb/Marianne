import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  base: './',
  resolve: {
    alias: {
      '$lib': path.resolve(__dirname, './src/renderer/lib')
    }
  },
  build: {
    outDir: 'dist/renderer',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, 'index.html'),
        splash: path.resolve(__dirname, 'src/renderer/splash.html')
      }
    }
  },
  server: {
    port: 5173
  }
});
