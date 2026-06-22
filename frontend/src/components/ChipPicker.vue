<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useUiStore } from '../stores/ui'
import { useLiveriesStore } from '../stores/liveries'

const ui = useUiStore()
const store = useLiveriesStore()
const newValue = ref('')
const inputRef = ref<HTMLInputElement | null>(null)

const ctx = computed(() => ui.chipPicker)
const isTag = computed(() => ctx.value?.type === 'tag')
const title = computed(() => (isTag.value ? 'Add a tag' : 'Add a collection'))

// Existing chip values across the catalog, minus the ones already on this livery.
const options = computed(() => {
  if (!ctx.value) return []
  const livery = store.byId(ctx.value.liveryId)
  const onLivery = new Set(isTag.value ? livery?.tags : livery?.collections)
  const all = isTag.value ? store.allTagValues() : store.allCollectionValues()
  return all.filter((v) => !onLivery.has(v))
})

function add(value: string) {
  const c = ctx.value
  if (!c || !value.trim()) return
  if (c.type === 'tag') store.addTag(c.liveryId, value.trim())
  else store.addCollection(c.liveryId, value.trim())
  ui.markCardDirty(c.liveryId)
  ui.closeChipPicker()
}
function createNew() {
  add(newValue.value)
  newValue.value = ''
}

watch(ctx, async (c) => {
  if (c) {
    newValue.value = ''
    await nextTick()
    inputRef.value?.focus()
  }
})
</script>

<template>
  <div class="image-picker" :class="{ open: !!ctx }" @click.self="ui.closeChipPicker()">
    <div class="image-picker-panel">
      <div class="image-picker-head">
        <span>{{ title }}</span>
        <button class="image-picker-close" aria-label="Close" @click="ui.closeChipPicker()">×</button>
      </div>
      <div class="chip-picker-grid">
        <button v-for="v in options" :key="v" class="tag chip" @click="add(v)">{{ v }}</button>
        <p v-if="!options.length" style="color: var(--steel); font-size: 13px; margin: 4px 0;">
          No existing values — create one below.
        </p>
      </div>
      <div class="chip-picker-new">
        <input ref="inputRef" v-model="newValue" type="text" placeholder="Create a new one…" @keydown.enter="createNew" />
        <button @click="createNew">Add</button>
      </div>
    </div>
  </div>
</template>
