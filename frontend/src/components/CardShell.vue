<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue'

const failed = ref(false)
const errorMsg = ref('')

onErrorCaptured((err) => {
  failed.value = true
  errorMsg.value = err instanceof Error ? err.message : String(err)
  return false
})
</script>

<template>
  <div v-if="failed" class="card card--error">
    <p class="card--error-msg">This card failed to render.</p>
    <p class="card--error-detail">{{ errorMsg }}</p>
  </div>
  <slot v-else />
</template>

<style scoped>
.card--error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  gap: 6px;
}
.card--error-msg {
  color: var(--danger);
  font-weight: 600;
}
.card--error-detail {
  color: var(--muted);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
}
</style>
