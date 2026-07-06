<script setup lang="ts">
import { useToastsStore } from '../stores/toasts'
const toasts = useToastsStore()
</script>

<template>
  <Teleport to="body">
    <div class="ts-stack">
      <TransitionGroup name="ts-slide">
        <div
          v-for="toast in toasts.toasts"
          :key="toast.id"
          class="ts-panel"
          :class="{ 'ts-panel--fading': toast.fadingOut }"
        >
          <div class="ts-header">
            <span class="ts-title">{{ toast.title }}</span>
            <button class="ts-close" @click="toasts.dismiss(toast.id)">×</button>
          </div>
          <div class="ts-items">
            <div
              v-for="item in toast.items"
              :key="item.id"
              class="ts-item"
              :class="'ts-item--' + item.status"
            >
              <span class="ts-dot" />
              <span class="ts-text">{{ item.text }}</span>
              <span v-if="item.detail" class="ts-detail">{{ item.detail }}</span>
            </div>
          </div>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.ts-stack {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 900;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}

.ts-panel {
  pointer-events: all;
  width: 280px;
  background: var(--panel-bg, #1a1a1a);
  border: 1px solid var(--panel-edge, #333);
  border-radius: 6px;
  overflow: hidden;
  transition: opacity .5s ease;
}
.ts-panel--fading { opacity: 0; pointer-events: none; }

.ts-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px 5px;
  border-bottom: 1px solid var(--panel-edge, #333);
}
.ts-title {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: .08em;
  text-transform: uppercase;
  color: var(--fg);
}
.ts-close {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 14px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}
.ts-close:hover { color: var(--fg); }

.ts-items {
  display: flex;
  flex-direction: column;
  padding: 6px 0 8px;
}
.ts-item {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 3px 10px;
  font: 10px/1.4 'JetBrains Mono', monospace;
}
.ts-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--muted);
}
.ts-item--processing .ts-dot {
  background: var(--accent);
  animation: ts-pulse 1s ease-in-out infinite;
}
.ts-item--done .ts-dot { background: #4a9; }
.ts-item--error .ts-dot { background: #c44; }

.ts-text { color: var(--fg); flex: 1; }
.ts-item--done .ts-text { color: var(--muted); }
.ts-item--error .ts-text { color: #e07070; }

.ts-detail {
  color: var(--muted);
  font-size: 9px;
  flex-shrink: 0;
}
.ts-item--done .ts-detail { color: #4a9; }

@keyframes ts-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: .3; }
}

.ts-slide-enter-active { transition: all .2s ease; }
.ts-slide-leave-active { transition: all .4s ease; }
.ts-slide-enter-from { opacity: 0; transform: translateX(20px); }
.ts-slide-leave-to  { opacity: 0; transform: translateX(20px); }
</style>
