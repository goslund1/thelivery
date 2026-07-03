<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { hideTip, refreshTip } from '../composables/tooltip'

const ui = useUiStore()
const modal = useModalStore()
const editCount = computed(() => ui.editCount)
const hasUnsaved = computed(() => ui.dirtyIds.size > 0)

function onExit() {
  hideTip()
  ui.requestExit()
}

// Cycle through every individually-typed field across all dirty cards.
const cycleIndex = ref(0)
const lastJumpIndex = ref(-1)
let _highlightTimer: ReturnType<typeof setTimeout> | null = null

// When the user manually focuses a known edit field, sync the pill position.
watch(() => ui.currentEditIndex, (idx) => {
  if (idx >= 0) lastJumpIndex.value = idx
})

// Reset counters when saved cards remove entries from the edit list.
watch(editCount, (next, prev) => {
  if (next < (prev ?? 0)) {
    cycleIndex.value = 0
    lastJumpIndex.value = -1
  }
})

async function onCycleDirty() {
  const count = ui.editCount
  if (!count) return

  const idx = cycleIndex.value % count
  lastJumpIndex.value = idx
  cycleIndex.value = (cycleIndex.value + 1) % count

  const entry = ui.getEditAt(idx)
  if (!entry) return
  const target = entry.el as HTMLElement

  // Open a collapsed <details> section before measuring position
  const details = target.closest('details') as HTMLDetailsElement | null
  if (details && !details.open) {
    details.querySelector('summary')?.click()
    await nextTick()
  }

  // Center in viewport — more predictable than scrollIntoView on a tall page
  const rect = target.getBoundingClientRect()
  const midpoint = rect.top + rect.height / 2
  const delta = midpoint - window.innerHeight / 2
  if (Math.abs(delta) > 4) {
    window.scrollBy({ top: delta, behavior: 'smooth' })
  }

  // Focus and restore cursor
  if (typeof target.focus === 'function') {
    target.focus({ preventScroll: true })
    if (entry.range) {
      try {
        const sel = window.getSelection()
        sel?.removeAllRanges()
        sel?.addRange(entry.range)
      } catch {
        // Range stale — leave cursor where focus placed it
      }
    }
  }

  // Pulse gold highlight ring
  if (_highlightTimer !== null) clearTimeout(_highlightTimer)
  target.classList.remove('edit-jump-target')
  void target.offsetWidth // force reflow to restart animation
  target.classList.add('edit-jump-target')
  _highlightTimer = setTimeout(() => {
    target.classList.remove('edit-jump-target')
    _highlightTimer = null
  }, 1600)

  refreshTip('Jump to next edit')
}

const pillLabel = computed(() => {
  if (lastJumpIndex.value < 0) return `${editCount.value} edit${editCount.value === 1 ? '' : 's'}`
  return `Edit ${lastJumpIndex.value + 1} / ${editCount.value}`
})
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
  left: 50%;
  transform: translateX(-50%);
  z-index: 900;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px 7px 18px;
  background: color-mix(in srgb, var(--panel) 80%, transparent);
  border: 1px solid var(--gold);
  border-radius: 999px;
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  color: var(--gold);
  white-space: nowrap;
  pointer-events: auto;
  box-shadow: 0 2px 16px rgba(0,0,0,0.4);
}
.edit-pill-label strong {
  font-weight: 700;
}
.edit-pill-save {
  background: none;
  border: 1px solid color-mix(in srgb, var(--gold) 60%, transparent);
  border-radius: 999px;
  color: var(--gold);
  font-family: inherit;
  font-size: 10px;
  letter-spacing: 0.06em;
  padding: 3px 12px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.edit-pill-save:hover {
  background: var(--gold);
  color: #111;
  border-color: var(--gold);
}

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
  color: var(--gold);
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
  background: #9a7400;
  border-color: #b89000;
  transform: none;
  box-shadow: none;
}
.tl-btn svg {
  flex-shrink: 0;
}

.tl-red {
  background: #8b1a12;
  border-color: #c0392b;
  color: #fff;
}
.tl-red:hover {
  background: #e82810;
  border-color: #ff4830;
  box-shadow: 0 0 12px rgba(232,40,16,0.75);
}
.tl-red svg {
  width: 14px;
  height: 14px;
}

.tl-yellow {
  background: #7a5800;
  border-color: #a07800;
  color: #fff;
}
.tl-yellow:hover {
  background: #ffc200;
  border-color: #ffe870;
  box-shadow: 0 0 16px rgba(255,194,0,0.85);
}
.icon-ibeam {
  width: 8px;
  height: 16px;
}

.tl-green {
  background: #0f4020;
  border-color: #1e7a3a;
}
.tl-green:hover {
  background: #16c24a;
  border-color: #34e068;
  box-shadow: 0 0 12px rgba(22,194,74,0.75);
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
