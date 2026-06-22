<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { registerTip } from '../composables/tooltip'

// The single shared tooltip element. Registers its DOM nodes with the tooltip
// module so the v-tip directive can drive it from anywhere.
const tip = ref<HTMLElement | null>(null)
const inner = ref<HTMLElement | null>(null)

onMounted(() => {
  if (tip.value && inner.value) registerTip(tip.value, inner.value)
})
</script>

<template>
  <div class="custom-tip" ref="tip">
    <span class="custom-tip-inner" ref="inner"></span>
  </div>
</template>
