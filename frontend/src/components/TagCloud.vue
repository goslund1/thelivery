<script setup lang="ts">
import type { Livery } from '../types'
import { useLiveriesStore } from '../stores/liveries'
import { useUiStore } from '../stores/ui'

const props = defineProps<{ livery: Livery }>()
const store = useLiveriesStore()
const ui = useUiStore()

const emit = defineEmits<{ buildIt: [] }>()

function removeTag(t: string) {
  store.removeTag(props.livery.id, t)
  ui.markCardDirty(props.livery.id)
}
</script>

<template>
  <div class="tag-cloud">
    <span
      v-for="t in livery.tags"
      :key="t"
      class="tag chip"
      data-chip-type="tag"
    >{{ t }}<button class="chip-remove" type="button" @click="removeTag(t)">×</button></span>
    <button class="chip-add" data-chip-type="tag" type="button" @click="ui.openChipPicker(livery.id, 'tag')">+</button>
    <a class="build-it" :href="`#recipe-${livery.id}`" @click="emit('buildIt')">Build It →</a>
  </div>
</template>
