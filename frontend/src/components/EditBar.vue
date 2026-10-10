<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { hideTip } from '../composables/tooltip'
import { useEditCycle } from '../composables/editCycle'

const ui = useUiStore()
const modal = useModalStore()
const editCount = computed(() => ui.editCount)
const hasUnsaved = computed(() => ui.dirtyIds.size > 0)

function onExit() {
  hideTip()
  ui.requestExit()
}

// Jump-to-next-edit lives in a shared composable so the phone drawer uses the same position.
const { onCycleDirty, pillLabel } = useEditCycle()
</script>

<template>
  <!-- Pill at bottom-center showing edit position and Save All -->
  <div class="edit-pill" v-show="ui.isEditing && editCount > 0">
    <span class="edit-pill-label">{{ pillLabel }}</span>
    <button class="edit-pill-save" type="button" @click="ui.saveAllDirty">Save All</button>
  </div>

  <div class="tl-dock" v-show="ui.isEditing">
    <span class="tl-mode-label">EDIT</span>

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

    <!-- Yellow: cycle through every edited field — i-cursor signals "you're editing" -->
    <button
      class="tl-btn tl-yellow"
      :class="{ 'tl-pulse': hasUnsaved, 'tl-idle': !hasUnsaved }"
      aria-label="Jump to next edit"
      v-tip="'Jump to next edit'"
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
      @click="modal.openNewCard()"
    >
      <svg class="icon-arrow" viewBox="0 0 20 20" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M 9 2 L 2 2 L 2 18 L 18 18 L 18 9"/>
        <line x1="10" y1="5.5" x2="17" y2="5.5"/>
        <line x1="13.5" y1="2.5" x2="13.5" y2="8.5"/>
      </svg>
    </button>

    <span class="tl-mode-label tl-mode-bottom">MODE</span>
  </div>
</template>

<style scoped>
.edit-pill {
  position: fixed;
  bottom: 18px;
  bottom: calc(18px + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 900;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px 7px 18px;
  background: color-mix(in srgb, var(--panel) 80%, transparent);
  border: 1px solid var(--accent);
  border-radius: 999px;
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  color: var(--accent);
  white-space: nowrap;
  pointer-events: auto;
  box-shadow: 0 2px 16px rgba(0,0,0,0.4);
}
.edit-pill-label strong {
  font-weight: 700;
}
.edit-pill-save {
  background: none;
  border: 1px solid color-mix(in srgb, var(--accent) 60%, transparent);
  border-radius: 999px;
  color: var(--accent);
  font-family: inherit;
  font-size: 10px;
  letter-spacing: 0.06em;
  padding: 3px 12px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.edit-pill-save:hover {
  background: var(--accent);
  color: var(--ink);
  border-color: var(--accent);
}

.tl-dock {
  position: fixed;
  right: max(0px, calc((100vw - 980px) / 2));
  bottom: 60px;
  bottom: calc(60px + env(safe-area-inset-bottom));
  z-index: 900;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 8px 10px;
  background: color-mix(in srgb, var(--panel) 69%, transparent);
  border: 1px solid var(--panel-edge);
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
  color: var(--accent);
  opacity: 0.75;
  pointer-events: none;
  width: 100%;
  text-align: center;
  padding-bottom: 2px;
  border-bottom: 1px solid var(--panel-edge);
}
.tl-mode-bottom {
  padding-bottom: 0;
  padding-top: 2px;
  border-bottom: none;
  border-top: 1px solid var(--panel-edge);
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
  transition: background 0.15s ease, border-color 0.15s ease, transform 0.12s ease, box-shadow 0.15s ease;
  flex-shrink: 0;
}
.tl-btn:hover {
  transform: scale(1.08);
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
.tl-yellow.tl-idle {
  cursor: default;
}
.tl-yellow.tl-idle:hover {
  background: color-mix(in srgb, var(--accent) 40%, transparent);
  border-color: var(--accent);
  transform: none;
  box-shadow: none;
}
.tl-btn svg {
  flex-shrink: 0;
}

.tl-red {
  background: color-mix(in srgb, var(--danger) 45%, transparent);
  border-color: var(--danger);
  color: #fff;
}
.tl-red:hover {
  background: var(--danger-bright);
  border-color: var(--danger-bright);
  box-shadow: 0 0 12px color-mix(in srgb, var(--danger-bright) 60%, transparent);
}
.tl-red svg {
  width: 14px;
  height: 14px;
}

.tl-yellow {
  background: color-mix(in srgb, var(--accent) 35%, transparent);
  border-color: var(--accent);
  color: #fff;
}
.tl-yellow:hover {
  background: var(--accent);
  border-color: var(--accent);
  box-shadow: 0 0 16px color-mix(in srgb, var(--accent) 60%, transparent);
}
.icon-ibeam {
  width: 8px;
  height: 16px;
}

.tl-green {
  background: color-mix(in srgb, var(--success) 40%, transparent);
  border-color: var(--success);
}
.tl-green:hover {
  background: var(--success-bright);
  border-color: var(--success-bright);
  box-shadow: 0 0 12px color-mix(in srgb, var(--success-bright) 60%, transparent);
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

/* Touch (phones and iPads): see the touch block in styles/responsive.css */
@media (hover: none), (pointer: coarse) {
  .tl-btn {
    width: 44px;
    height: 44px;
  }
}

/* Phones (A2): the dock and pill would sit on the card; their actions are in the
   bottom bar instead (SideBug.vue). */
@media (max-width: 640px) {
  .edit-pill,
  .tl-dock { display: none; }
}
</style>
