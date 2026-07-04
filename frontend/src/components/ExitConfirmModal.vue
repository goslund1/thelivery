<script setup lang="ts">
import { useUiStore } from '../stores/ui'
const ui = useUiStore()
</script>

<template>
  <div class="conf-overlay" :class="{ open: ui.exitConfirmOpen }" @click.self="ui.cancelExit()">
    <div class="conf-panel">
      <div class="conf-head">
        <span>Unsaved Changes</span>
        <button class="conf-close" aria-label="Close" @click="ui.cancelExit()">×</button>
      </div>
      <p class="conf-body">You have unsaved changes. What would you like to do?</p>
      <div class="conf-actions">
        <button class="conf-btn conf-btn--confirm" :disabled="ui.saving" @click="ui.confirmSaveAndExit()">Save and Exit</button>
        <button class="conf-btn conf-btn--neutral" @click="ui.cancelExit()">Continue Editing</button>
        <button class="conf-btn conf-btn--discard" @click="ui.confirmDiscardAndExit()">Discard and Exit</button>
      </div>
    </div>
  </div>
</template>
