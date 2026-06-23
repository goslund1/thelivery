<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { hideTip } from '../composables/tooltip'

const ui = useUiStore()
const unsavedCount = computed(() => ui.dirtyIds.size)

function onExit() {
  hideTip()
  ui.requestExit()
}
</script>

<template>
  <div class="tl-dock" v-show="ui.isEditing">
    <!-- Edit mode indicator — pointer-events:none so nothing underneath is blocked -->
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

    <!-- Yellow: save — actual i-cursor icon (Font Awesome solid path, filled shape) -->
    <button
      class="tl-btn tl-yellow"
      :class="{ 'tl-pulse': unsavedCount > 0 }"
      :disabled="ui.saving"
      aria-label="Save changes"
      v-tip="() => ui.saving ? 'Saving…' : unsavedCount > 0 ? `Save changes (${unsavedCount})` : 'All saved'"
      @click="ui.saveAllDirty()"
    >
      <svg class="icon-ibeam" viewBox="0 0 256 512" fill="white">
        <path d="M.1 29.3C-1.4 47 11.7 62.4 29.3 63.9l8 .7C70.5 67.3 96 95 96 128.3L96 224l-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 95.7c0 33.3-25.5 61-58.7 63.8l-8 .7C11.7 449.6-1.4 465 .1 482.7s16.9 30.7 34.5 29.2l8-.7c34.1-2.8 64.2-18.9 85.4-42.9c21.2 24 51.2 40 85.4 42.9l8 .7c17.6 1.5 33.1-11.6 34.5-29.2s-11.6-33.1-29.2-34.5l-8-.7C185.5 444.7 160 417 160 383.7l0-95.7 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-95.7c0-33.3 25.5-61 58.7-63.8l8-.7c17.6-1.5 30.7-16.9 29.2-34.5S239-1.4 221.3 .1l-8 .7C179.2 3.6 149.2 19.7 128 43.7c-21.2-24-51.2-40-85.4-42.9l-8-.7C17-1.4 1.6 11.7 .1 29.3z"/>
      </svg>
    </button>

    <!-- Green: new card
         Shaft: vertical section up, then 45° diagonal.
         Head: solid filled triangle, tip pointing NE at 45°.
         Base midpoint of triangle = end of diagonal, so shaft connects cleanly.
         Plus: just left of the diagonal (upper) section. -->
    <button
      class="tl-btn tl-green"
      aria-label="New card"
      v-tip="'New card'"
      @click="ui.openNewCard()"
    >
      <svg class="icon-arrow" viewBox="0 0 16 16" fill="none" stroke="white" stroke-linecap="round">
        <!-- Section 1: vertical shaft -->
        <line x1="7" y1="15" x2="7" y2="9" stroke-width="2"/>
        <!-- Section 2: 45° diagonal shaft (Δx=3 Δy=-3) -->
        <line x1="7" y1="9" x2="10" y2="6" stroke-width="2"/>
        <!-- Arrowhead: solid triangle pointing NE.
             Tip at (14,2). Base midpoint at (10,6) = diagonal end.
             Direction (10,6)→(14,2) = (4,-4) = exactly 45°. -->
        <polygon points="14,2 9,5 11,7" fill="white" stroke="none"/>
        <!-- Plus: just left of the 45° section -->
        <line x1="1" y1="7" x2="5" y2="7" stroke-width="1.7"/>
        <line x1="3" y1="5" x2="3" y2="9" stroke-width="1.7"/>
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
  opacity: 0.5;
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
  width: 10px;
  height: 20px;
}

.tl-green {
  background: #1e7a3a;
  border-color: #27ae60;
}
.tl-green:hover {
  box-shadow: 0 0 8px rgba(39,174,96,0.5);
}
.icon-arrow {
  width: 16px;
  height: 16px;
}

.tl-pulse {
  animation: tl-pulse 1.8s ease-in-out infinite;
}
@keyframes tl-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(201,162,39,0.7); }
  50%       { box-shadow: 0 0 0 7px rgba(201,162,39,0); }
}
</style>
