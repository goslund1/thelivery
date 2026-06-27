<script setup lang="ts">
import { nextTick, watch } from 'vue'
import { useUiStore } from '../stores/ui'

// Replaces a native <details class="section"> from the original. Open state is
// a v-model so a parent can force it open (e.g. the "Build It" link), and it
// follows the per-type expand state driven by the side-bug section checkboxes
// and the global expand/collapse-all toggle.
const props = defineProps<{ sectionKey: string; label: string; domId?: string }>()
const open = defineModel<boolean>('open', { default: false })
const ui = useUiStore()

watch(() => ui.sectionExpanded[props.sectionKey], (v) => (open.value = v))

function onToggle(e: Event) {
  open.value = (e.target as HTMLDetailsElement).open
}

// Fired on <summary> click — BEFORE the native <details> toggle runs.
// Captures the summary's viewport position so we can compensate after the
// layout change, keeping the header visually pinned in place.
//
// Special case: if this is a collapse AND the summary is above the viewport
// (user scrolled deep into a tall section), snap the summary to the top of
// the window instead so the header is always visible after the section closes.
function onSummaryClick(e: Event) {
  const summary = e.currentTarget as HTMLElement
  const details = summary.closest('details') as HTMLDetailsElement
  const isCollapsing = details.open
  const topBefore = summary.getBoundingClientRect().top

  nextTick(() => {
    const topAfter = summary.getBoundingClientRect().top
    if (isCollapsing && topBefore < 0) {
      // Was scrolled into a tall section — bring the header to the top of the window.
      window.scrollBy(0, topAfter)
    } else {
      // Normal case — keep the summary pinned where it was.
      window.scrollBy(0, topAfter - topBefore)
    }
  })
}
</script>

<template>
  <details
    class="section"
    :data-section="sectionKey"
    :id="domId"
    :open="open"
    @toggle="onToggle"
  >
    <summary title="Click to expand or collapse" @click="onSummaryClick">
      <span class="section-label">{{ label }}</span> <span class="chev"></span>
    </summary>
    <slot />
  </details>
</template>
