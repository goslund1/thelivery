import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './styles/catalog.css'
import App from './App.vue'
import { vTip, vTipUp } from './composables/tooltip'
import { vScrollContain } from './directives/scrollContain'

createApp(App).use(createPinia()).directive('tip', vTip).directive('tip-up', vTipUp).directive('scroll-contain', vScrollContain).mount('#app')
