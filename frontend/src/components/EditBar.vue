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

    <!-- Yellow: save changes (cursor = active editing session indicator) -->
    <button
      class="tl-btn tl-yellow"
      :class="{ 'tl-pulse': unsavedCount > 0 }"
      :disabled="ui.saving"
      aria-label="Save changes"
      v-tip="() => ui.saving ? 'Saving…' : unsavedCount > 0 ? `Save changes (${unsavedCount})` : 'All saved'"
      @click="ui.saveAllDirty()"
    >
      <svg viewBox="0 0 11 15" fill="currentColor">
        <path d="M1 0 L1 11 L3.8 8.2 L5.8 13.5 L7.8 12.5 L5.8 7.2 L9.2 7.2 Z"/>
      </svg>
    </button>

    <!-- Green: new card (corner arrow + plus = create and add to catalog) -->
    <button
      class="tl-btn tl-green"
      aria-label="New card"
      v-tip="'New card'"
      @click="ui.openNewCard()"
    >
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <!-- L-shaped corner arrow: right-down then turns left -->
        <path d="M10 2 L10 9 L4 9"/>
        <!-- arrowhead pointing left -->
        <path d="M6.5 7 L4 9 L6.5 11"/>
        <!-- plus sign: create/add -->
        <line x1="2" y1="1" x2="2" y2="5"/>
        <line x1="0" y1="3" x2="4" y2="3"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.tl-dock {
  position: fixed;
  right: 0;
  bottom: 60px;
  z-index: 900;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 10px 8px;
  background: var(--panel-bg, rgba(30,30,30,0.92));
  border: 1px solid var(--panel-edge, rgba(255,255,255,0.12));
  border-right: none;
  border-radius: 10px 0 0 10px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
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
  width: 14px;
  height: 14px;
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

.tl-yellow {
  background: #a07000;
  border-color: var(--gold, #c9a227);
  color: #1a1200;
}
.tl-yellow:hover {
  box-shadow: 0 0 8px rgba(201,162,39,0.5);
}
.tl-yellow svg {
  width: 11px;
  height: 15px;
}

.tl-green {
  background: #1e7a3a;
  border-color: #27ae60;
  color: #d4f5e2;
}
.tl-green:hover {
  box-shadow: 0 0 8px rgba(39,174,96,0.5);
}

.tl-pulse {
  animation: tl-pulse 1.8s ease-in-out infinite;
}
@keyframes tl-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(201,162,39,0.7); }
  50%       { box-shadow: 0 0 0 7px rgba(201,162,39,0); }
}
</style>
