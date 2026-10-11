import { dirname, resolve } from 'node:path'
import { defineConfig, searchForWorkspaceRoot } from 'vite'

export default defineConfig({
  base: './',
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
    fs: {
      allow: [
        searchForWorkspaceRoot(process.cwd()),
        '/home/katsuya/work/wasmer-js',
      ]
    }
  },
  optimizeDeps: {
    exclude: [
      '/home/katsuya/work/wasmer-js',
      '@wasmer/sdk',
      "./src/main.wasm",
      "@rollup/browser",
    ],
  },
  plugins: [
    {
      name: 'wasm-content-type',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url.endsWith('.wasm')) {
            res.setHeader('Content-Type', 'application/wasm');
            res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
            res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
          }
          next();
        });
      },
    },
  ]
})
