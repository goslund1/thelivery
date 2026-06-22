<script setup lang="ts">
import { watch } from 'vue'
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
</script>

<template>
  <details
    class="section"
    :data-section="sectionKey"
    :id="domId"
    :open="open"
    @toggle="onToggle"
  >
    <summary title="Click to expand or collapse">
      <span class="section-label">{{ label }}</span> <span class="chev"></span>
    </summary>
    <slot />
  </details>
</template>
