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

    <!-- Yellow: save changes — text-insertion I-beam = "you're in an editing session" -->
    <button
      class="tl-btn tl-yellow"
      :class="{ 'tl-pulse': unsavedCount > 0 }"
      :disabled="ui.saving"
      aria-label="Save changes"
      v-tip="() => ui.saving ? 'Saving…' : unsavedCount > 0 ? `Save changes (${unsavedCount})` : 'All saved'"
      @click="ui.saveAllDirty()"
    >
      <svg viewBox="0 0 10 16" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round">
        <line x1="1" y1="2"  x2="9" y2="2"/>
        <line x1="5" y1="2"  x2="5" y2="14"/>
        <line x1="1" y1="14" x2="9" y2="14"/>
      </svg>
    </button>

    <!-- Green: new card — curve rising to 45° upper-right, plus left of upper arc -->
    <button
      class="tl-btn tl-green"
      aria-label="New card"
      v-tip="'New card'"
      @click="ui.openNewCard()"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="white" stroke-linecap="round" stroke-linejoin="round">
        <!-- Smooth curve: rises from bottom, turns and points 45° upper-right -->
        <path d="M 8 15 C 8 10 9 7 13 3" stroke-width="2"/>
        <!-- Arrowhead at tip (13,3) pointing NE — wings perpendicular to 45° -->
        <polyline points="11,1 13,3 15,5" stroke-width="2"/>
        <!-- Plus: just to the left of the upper portion of the curve -->
        <line x1="1" y1="7" x2="5" y2="7" stroke-width="1.7"/>
        <line x1="3" y1="5" x2="3" y2="9" stroke-width="1.7"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.tl-dock {
  position: fixed;
  /* Align to the right edge of the 980px catalog column, not the viewport edge.
     When the viewport is narrower than 980px the catalog goes edge-to-edge, so clamp to 0. */
  right: max(0px, calc((100vw - 980px) / 2));
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
  color: #fff;
}
.tl-yellow:hover {
  box-shadow: 0 0 8px rgba(201,162,39,0.5);
}
.tl-yellow svg {
  width: 10px;
  height: 16px;
}

.tl-green {
  background: #1e7a3a;
  border-color: #27ae60;
  color: #d4f5e2;
}
.tl-green svg {
  width: 16px;
  height: 16px;
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
