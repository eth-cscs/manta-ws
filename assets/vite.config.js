/* import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify, { transformAssetUrls } from 'vite-plugin-vuetify' 

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue({template: { transformAssetUrls }}), vuetify({ autoImport: true }), { styles: { configFile: 'src/settings.scss' }}, { styles: 'expose' }],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  }
}) */

import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'

export default {
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
  ],
}
  
