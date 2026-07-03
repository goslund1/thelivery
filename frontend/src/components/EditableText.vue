<script setup lang="ts">
import { ref, watch, onMounted, inject } from 'vue'
import { useUiStore } from '../stores/ui'
import { MarkDirtyKey, CardIdKey } from '../keys'

// A text node that becomes contenteditable in edit mode and two-way binds to a
// string. Content is written to the DOM imperatively (not via template
// interpolation) so typing never resets the caret. Replaces the original app's
// contentEditable fields + input-event dirty tracking.
const props = defineProps<{ modelValue: string; tag?: string }>()
const emit = defineEmits<{ 'update:modelValue': [string] }>()
const ui = useUiStore()
const markDirty = inject(MarkDirtyKey, () => {})
const cardId = inject(CardIdKey, null)
const el = ref<HTMLElement | null>(null)

function onInput() {
  emit('update:modelValue', el.value?.innerText ?? '')
  markDirty()
  if (cardId && el.value) ui.addToEditList(cardId, el.value)
}
function onFocus() {
  if (el.value) ui.setFocusedEdit(el.value)
}
function onBlur() {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0 || !el.value?.contains(sel.anchorNode)) return
  if (el.value) ui.saveRange(el.value, sel.getRangeAt(0).cloneRange())
}

onMounted(() => {
  if (el.value) el.value.innerText = props.modelValue
})

// Sync external changes into the DOM only when this field isn't being edited.
watch(
  () => props.modelValue,
  (v) => {
    if (el.value && document.activeElement !== el.value && el.value.innerText !== v) {
      el.value.innerText = v
    }
  },
)
</script>

<template>
  <component
    :is="tag || 'span'"
    ref="el"
    :contenteditable="ui.isEditing"
    class="editable"
    @input="onInput"
    @focus="onFocus"
    @blur="onBlur"
  />
</template>
