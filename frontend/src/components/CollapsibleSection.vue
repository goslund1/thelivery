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

// Capture summary position before the native toggle, compensate after.
// No special-case for "above viewport" — a user can't click an off-screen summary,
// so topBefore is always >= 0 for real clicks. Global collapse-all is handled in SideBug.
function onSummaryClick(e: Event) {
  const summary = e.currentTarget as HTMLElement
  const topBefore = summary.getBoundingClientRect().top
  nextTick(() => {
    window.scrollBy(0, summary.getBoundingClientRect().top - topBefore)
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
