<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { hideTip } from '../composables/tooltip'
const ui = useUiStore()

// Saving is per-card now; this bar only offers the exit control plus a count of
// cards with unsaved changes. Exiting with any unsaved card opens the prompt.
const unsavedCount = computed(() => ui.dirtyIds.size)

// Exit may open the unsaved-changes modal → hide the tooltip immediately (Rule A).
function onExit() {
  hideTip()
  ui.requestExit()
}
</script>

<template>
  <div class="edit-action-row" :class="{ open: ui.isEditing }" v-show="ui.isEditing">
    <button class="exit-edit-btn" aria-label="Exit edit mode" v-tip="'Exit edit mode'" @click="onExit">×</button>
    <button class="new-card-btn" @click="ui.openNewCard()">+ New Card</button>
    <button
      class="save-changes-btn"
      :class="{ 'has-changes': unsavedCount > 0 }"
      :disabled="unsavedCount === 0 || ui.saving"
      @click="ui.saveAllDirty()"
    >{{ ui.saving ? 'Saving…' : unsavedCount > 0 ? `Save Changes (${unsavedCount})` : 'All Saved' }}</button>
  </div>
</template>

<style scoped>
.new-card-btn {
  background: #2d7d52;
  border: 1px solid #3a9e68;
  color: #e8f5ee;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  white-space: nowrap;
}
.new-card-btn:hover {
  background: #358f5e;
  border-color: #4ab87a;
}
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
