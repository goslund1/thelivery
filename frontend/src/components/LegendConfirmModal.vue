<script setup lang="ts">
import { useUiStore } from '../stores/ui'
const ui = useUiStore()
</script>

<template>
  <div class="conf-overlay" :class="{ open: ui.legendConfirmOpen }" @click.self="ui.cancelLegendUpdate()">
    <div class="conf-panel">
      <div class="conf-head">
        <span>Update Card Defaults</span>
        <button class="conf-close" aria-label="Close" @click="ui.cancelLegendUpdate()">×</button>
      </div>
      <p class="conf-body">Changes have been made to Card #000. Saving will update the default values for all new cards.</p>
      <div class="conf-actions">
        <button class="conf-btn conf-btn--confirm" @click="ui.confirmLegendUpdate">Yes, Update Defaults</button>
        <button class="conf-btn conf-btn--neutral" @click="ui.cancelLegendUpdate()">Cancel</button>
      </div>
    </div>
  </div>
</template>
