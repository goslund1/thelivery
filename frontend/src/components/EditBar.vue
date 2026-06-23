<script setup lang="ts">
import { computed, ref } from 'vue'
import { useUiStore } from '../stores/ui'
import { hideTip, refreshTip } from '../composables/tooltip'

const ui = useUiStore()
const unsavedCount = computed(() => ui.dirtyIds.size)

function onExit() {
  hideTip()
  ui.requestExit()
}

// Cycle through dirty cards on each click, scrolling each into view in turn.
const cycleIndex = ref(0)
function onCycleDirty() {
  const ids = [...ui.dirtyIds]
  if (!ids.length) return
  const id = ids[cycleIndex.value % ids.length]
  cycleIndex.value = (cycleIndex.value + 1) % ids.length
  const el = document.getElementById(`card-${id}`)
  if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  refreshTip(cycleTooltip.value)
}

const cycleTooltip = computed(() => {
  const n = unsavedCount.value
  if (n === 0) return 'All saved'
  if (n === 1) return 'Jump to unsaved card'
  return `Jump to unsaved (${cycleIndex.value % n + 1} of ${n})`
})
</script>

<template>
  <div class="tl-dock" v-show="ui.isEditing">
    <span class="tl-mode-label">EDIT</span>

    <!-- Red: exit edit mode -->
    <button
      class="tl-btn tl-red"
      aria-label="Exit edit mode"
      v-tip="'Exit edit mode'"
      @click="onExit"
    >
      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
        <line x1="2" y1="2" x2="10" y2="10"/>
        <line x1="10" y1="2" x2="2" y2="10"/>
      </svg>
    </button>

    <!-- Yellow: cycle through unsaved cards — i-cursor signals "you're editing" -->
    <button
      class="tl-btn tl-yellow"
      :class="{ 'tl-pulse': unsavedCount > 0 }"
      :disabled="unsavedCount === 0"
      aria-label="Jump to unsaved card"
      v-tip="() => cycleTooltip"
      @click="onCycleDirty"
    >
      <svg class="icon-ibeam" viewBox="0 0 256 512" fill="white">
        <path d="M.1 29.3C-1.4 47 11.7 62.4 29.3 63.9l8 .7C70.5 67.3 96 95 96 128.3L96 224l-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 95.7c0 33.3-25.5 61-58.7 63.8l-8 .7C11.7 449.6-1.4 465 .1 482.7s16.9 30.7 34.5 29.2l8-.7c34.1-2.8 64.2-18.9 85.4-42.9c21.2 24 51.2 40 85.4 42.9l8 .7c17.6 1.5 33.1-11.6 34.5-29.2s-11.6-33.1-29.2-34.5l-8-.7C185.5 444.7 160 417 160 383.7l0-95.7 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-95.7c0-33.3 25.5-61 58.7-63.8l8-.7c17.6-1.5 30.7-16.9 29.2-34.5S239-1.4 221.3 .1l-8 .7C179.2 3.6 149.2 19.7 128 43.7c-21.2-24-51.2-40-85.4-42.9l-8-.7C17-1.4 1.6 11.7 .1 29.3z"/>
      </svg>
    </button>

    <!-- Green: new card -->
    <button
      class="tl-btn tl-green"
      aria-label="New card"
      v-tip="'New card'"
      @click="ui.openNewCard()"
    >
      <svg class="icon-arrow" viewBox="0 0 512 512" fill="white">
        <path d="M352 0c-12.9 0-24.6 7.8-29.6 19.8s-2.2 25.7 6.9 34.9L370.7 96 201.4 265.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L416 141.3l41.4 41.4c9.2 9.2 22.9 11.9 34.9 6.9s19.8-16.6 19.8-29.6l0-128c0-17.7-14.3-32-32-32L352 0zM80 32C35.8 32 0 67.8 0 112L0 432c0 44.2 35.8 80 80 80l320 0c44.2 0 80-35.8 80-80l0-112c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 112c0 8.8-7.2 16-16 16L80 448c-8.8 0-16-7.2-16-16l0-320c0-8.8 7.2-16 16-16l112 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80 32z"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.tl-dock {
  position: fixed;
  right: max(0px, calc((100vw - 980px) / 2));
  bottom: 60px;
  z-index: 900;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 8px 10px;
  background: var(--panel-bg, rgba(30,30,30,0.92));
  border: 1px solid var(--panel-edge, rgba(255,255,255,0.12));
  border-right: none;
  border-radius: 10px 0 0 10px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}

.tl-mode-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 8px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--gold, #c9a227);
  opacity: 0.75;
  pointer-events: none;
  padding-bottom: 2px;
  border-bottom: 1px solid var(--panel-edge, rgba(255,255,255,0.1));
  width: 100%;
  text-align: center;
}

.tl-btn {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: filter 0.15s ease, transform 0.12s ease, box-shadow 0.15s ease;
  flex-shrink: 0;
}
.tl-btn:hover {
  filter: brightness(1.18);
  transform: scale(1.1);
}
.tl-btn:active {
  transform: scale(0.96);
}
.tl-btn:disabled {
  opacity: 0.35;
  cursor: default;
  transform: none;
  filter: none;
}
.tl-btn svg {
  flex-shrink: 0;
}

.tl-red {
  background: #c0392b;
  border-color: #e74c3c;
  color: #fff;
}
.tl-red:hover {
  box-shadow: 0 0 8px rgba(231,76,60,0.55);
}
.tl-red svg {
  width: 14px;
  height: 14px;
}

.tl-yellow {
  background: #a07000;
  border-color: var(--gold, #c9a227);
}
.tl-yellow:hover {
  box-shadow: 0 0 8px rgba(201,162,39,0.5);
}
.icon-ibeam {
  width: 8px;
  height: 16px;
}

.tl-green {
  background: #1e7a3a;
  border-color: #27ae60;
}
.tl-green:hover {
  box-shadow: 0 0 8px rgba(39,174,96,0.5);
}
.icon-arrow {
  width: 17px;
  height: 17px;
}

.tl-pulse {
  animation: tl-pulse 1.8s ease-in-out infinite;
}
@keyframes tl-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(201,162,39,0.7); }
  50%       { box-shadow: 0 0 0 7px rgba(201,162,39,0); }
}
</style>
