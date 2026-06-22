<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '../stores/ui'
const ui = useUiStore()

// Saving is per-card now; this bar only offers the exit control plus a count of
// cards with unsaved changes. Exiting with any unsaved card opens the prompt.
const unsavedCount = computed(() => ui.dirtyIds.size)
</script>

<template>
  <div class="edit-action-row" :class="{ open: ui.isEditing }" v-show="ui.isEditing">
    <button class="exit-edit-btn" aria-label="Exit edit mode" @click="ui.requestExit()">×</button>
    <span v-if="unsavedCount" class="edit-unsaved-count">
      {{ unsavedCount }} card{{ unsavedCount === 1 ? '' : 's' }} unsaved
    </span>
  </div>
</template>

<style scoped>
.edit-unsaved-count {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--gold);
  align-self: center;
  white-space: nowrap;
}
</style>
