<script setup lang="ts">
import { computed } from 'vue'
import type { TextSection } from '../types'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'

const props = defineProps<{ cardId: string; section: TextSection }>()
const ui = useUiStore()

// Inspiration keeps its italic styling (.anecdote-text); other text sections use
// the plain .gutter-text class — matching the original.
const bodyClass = computed(() => (props.section.key === 'inspiration' ? 'anecdote-text' : 'gutter-text'))
</script>

<template>
  <div class="section-body gutter-layout">
    <div class="gutter-figure" :class="{ 'has-image': section.figurePath }" :data-group="cardId">
      <img
        v-if="section.figurePath"
        class="gutter-figure-img"
        :src="section.figurePath"
        @click="ui.openLightbox(section.figurePath!)"
      />
      <button class="change-image-btn" type="button" @click="ui.openImagePicker(cardId, section.key)">Change Image</button>
    </div>
    <EditableText tag="div" :class="bodyClass" v-model="section.body" />
  </div>
</template>
