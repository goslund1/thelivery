<script setup lang="ts">
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'

// recipeKey is the dom-id key of the card's forza_recipe section, if any.
const props = defineProps<{ card: Card; recipeKey?: string }>()
const store = useCardsStore()
const ui = useUiStore()

const emit = defineEmits<{ buildIt: [] }>()

function removeTag(t: string) {
  store.removeTag(props.card.id, t)
  ui.markCardDirty(props.card.id)
}
</script>

<template>
  <div class="tag-cloud">
    <span
      v-for="t in card.tags"
      :key="t"
      class="tag chip"
      data-chip-type="tag"
    >{{ t }}<button class="chip-remove" type="button" @click="removeTag(t)">×</button></span>
    <button class="chip-add" data-chip-type="tag" type="button" @click="ui.openChipPicker(card.id, 'tag')">+</button>
    <a v-if="recipeKey" class="build-it" :href="`#${recipeKey}-${card.id}`" v-tip="'Jump to the build/tune recipe'" @click="emit('buildIt')">Build It →</a>
  </div>
</template>
