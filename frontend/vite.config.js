import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// 家庭内网场景：后端默认 8080，前端 dev server 5173，通过 proxy 免跨域
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const backend = env.VITE_API_TARGET || 'http://127.0.0.1:8080'

  return {
    plugins: [vue()],
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url))
      }
    },
    server: {
      host: true,        // 监听 0.0.0.0，手机/平板可通过局域网 IP 访问
      port: Number(env.VITE_PORT) || 5173,
      strictPort: false,
      proxy: {
        '/api': {
          target: backend,
          changeOrigin: true
        }
      }
    },
    preview: {
      host: true,
      port: 4173
    },
    build: {
      outDir: 'dist',
      assetsDir: 'assets',
      sourcemap: false,
      chunkSizeWarningLimit: 900,
      rollupOptions: {
        output: {
          manualChunks: {
            vue: ['vue', 'vue-router', 'pinia'],
            vendor: ['axios']
          }
        }
      }
    }
  }
})
