<script setup lang="ts">
import { nextTick, watch } from 'vue'
import { useUiStore } from '../stores/ui'

const props = defineProps<{ sectionKey: string; label: string; domId?: string }>()
const open = defineModel<boolean>('open', { default: false })
const ui = useUiStore()

watch(() => ui.sectionExpanded[props.sectionKey], (v) => (open.value = v))

function onToggle(e: Event) {
  open.value = (e.target as HTMLDetailsElement).open
}

// Fired on <summary> click — BEFORE the native <details> toggle runs.
// Pattern mirrors SideBug's onToggleAll: capture position before, compensate after nextTick.
//
// Special case (collapse only): if the user scrolled deep into a tall section so the
// summary is above the viewport, snap to the top of the parent card instead.
// We capture the card's absolute document position HERE (before any DOM change) because
// collapsing a section inside a card doesn't move the card itself in the document —
// so getBoundingClientRect().top + scrollY is stable across the toggle.
function onSummaryClick(e: Event) {
  const summary = e.currentTarget as HTMLElement
  const details = summary.closest('details') as HTMLDetailsElement
  const isCollapsing = details.open
  const topBefore = summary.getBoundingClientRect().top

  // Capture card's absolute document top now, before anything changes.
  const card = summary.closest('.card') as HTMLElement | null
  const cardDocTop = card != null ? card.getBoundingClientRect().top + window.scrollY : null

  nextTick(() => {
    if (isCollapsing && topBefore < 0 && cardDocTop != null) {
      // Summary was above the viewport — scroll to the card's top.
      window.scrollTo({ top: cardDocTop })
    } else {
      // Normal case — keep the summary pinned at its current viewport position.
      window.scrollBy(0, summary.getBoundingClientRect().top - topBefore)
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
