import { defineConfig, mergeConfig, type Plugin } from 'vite';
import base from './vite.config';

/**
 * The web build for `diffity-server`: the same app, with `src-web/tauri-internals.ts` loaded first so its Tauri
 * calls go to the server. `pnpm build:web` → `dist-web/`.
 */
function loadTauriInternalsFirst(): Plugin {
  return {
    name: 'diffity-web-tauri-internals',
    transformIndexHtml: {
      order: 'pre',
      handler: (html) => html.replace('<script type="module"', '<script type="module" src="/src-web/tauri-internals.ts"></script>\n    <script type="module"'),
    },
  };
}

export default mergeConfig(
  base,
  defineConfig({
    plugins: [loadTauriInternalsFirst()],
    build: {
      outDir: 'dist-web',
      // Browsers, not the macOS WebKit the app pins.
      target: 'es2022',
    },
    server: {
      port: 1430,
      proxy: {
        '/api': { target: 'http://127.0.0.1:4318', ws: true },
      },
    },
  }),
);
