<script setup lang="ts">
import { useUiStore } from '../stores/ui'
const ui = useUiStore()
</script>

<template>
  <div class="image-picker" :class="{ open: ui.legendConfirmOpen }">
    <div class="image-picker-panel exit-confirm-panel">
      <div class="image-picker-head">
        <span>Update Card Defaults</span>
      </div>
      <p class="exit-confirm-text">Changes have been made to Card #000. Saving will update the default values for all new cards.</p>
      <div class="exit-confirm-options">
        <button id="legend-save-btn" @click="ui.confirmLegendUpdate">Yes, Update Defaults</button>
        <button id="legend-cancel-btn" @click="ui.cancelLegendUpdate">Cancel</button>
      </div>
    </div>
  </div>
</template>
