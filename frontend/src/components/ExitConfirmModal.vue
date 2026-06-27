<script setup lang="ts">
import { useUiStore } from '../stores/ui'
const ui = useUiStore()
</script>

<template>
  <div class="image-picker" :class="{ open: ui.exitConfirmOpen }" @click.self="ui.cancelExit()">
    <div class="image-picker-panel exit-confirm-panel">
      <div class="image-picker-head">
        <span>Unsaved Changes</span>
        <button class="image-picker-close" aria-label="Close" @click="ui.cancelExit()">×</button>
      </div>
      <p class="exit-confirm-text">You have unsaved changes. What would you like to do?</p>
      <div class="exit-confirm-options">
        <button id="exit-save-btn" :disabled="ui.saving" @click="ui.confirmSaveAndExit()">Save and Exit</button>
        <button id="exit-continue-btn" @click="ui.cancelExit()">Continue Editing</button>
        <button id="exit-discard-btn" @click="ui.confirmDiscardAndExit()">Discard and Exit</button>
      </div>
    </div>
  </div>
</template>
