import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './styles/catalog.css'
import App from './App.vue'
import { vTip } from './composables/tooltip'

createApp(App).use(createPinia()).directive('tip', vTip).mount('#app')
