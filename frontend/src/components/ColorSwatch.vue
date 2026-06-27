<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  modelValue?: string
  label?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [string | undefined] }>()

const inputEl = ref<HTMLInputElement | null>(null)

function open() { inputEl.value?.click() }

function onPick(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value)
}

function clear(e: MouseEvent) {
  e.stopPropagation()
  emit('update:modelValue', undefined)
}
</script>

<template>
  <button
    class="cswatch"
    :class="{ 'cswatch--set': !!modelValue }"
    :style="modelValue ? { background: modelValue, borderColor: modelValue } : {}"
    :title="label ?? 'Set color'"
    type="button"
    @click="open"
  >
    <input
      ref="inputEl"
      type="color"
      :value="modelValue ?? '#ffffff'"
      @change="onPick"
      @click.stop
    />
    <span v-if="modelValue" class="cswatch-x" @click.stop="clear">×</span>
  </button>
</template>

<style scoped>
.cswatch {
  position: relative;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1.5px dashed rgba(255,255,255,0.28);
  background: transparent;
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
  transition: transform 0.12s ease, border-color 0.12s ease, box-shadow 0.12s ease;
  outline: none;
}
.cswatch:hover {
  transform: scale(1.25);
  border-color: rgba(255,255,255,0.7);
}
.cswatch--set {
  border-style: solid;
  border-width: 2px;
}
.cswatch--set:hover {
  box-shadow: 0 0 6px currentColor;
}
input[type="color"] {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
  top: 0;
  left: 0;
}
.cswatch-x {
  position: absolute;
  top: -5px;
  right: -5px;
  width: 11px;
  height: 11px;
  background: #1a1a1a;
  border: 1px solid rgba(255,255,255,0.35);
  border-radius: 50%;
  font-size: 8px;
  line-height: 11px;
  text-align: center;
  color: #fff;
  cursor: pointer;
  display: none;
  z-index: 1;
}
.cswatch:hover .cswatch-x {
  display: block;
}
</style>
