<script setup lang="ts">
import { computed } from 'vue'
import type { TextSection } from '../types'
import { useModalStore } from '../stores/modal'
import EditableText from './EditableText.vue'

const props = defineProps<{ cardId: string; section: TextSection }>()
const modal = useModalStore()

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
        @click="modal.openLightbox(section.figurePath!)"
      />
      <span v-else class="gutter-figure-empty">Select image</span>
      <button class="change-image-btn" type="button" @click="modal.openImagePicker(cardId, section.key)">{{ section.figurePath ? 'Change Image' : 'Select Image' }}</button>
    </div>
    <EditableText tag="div" :class="bodyClass" v-model="section.body" />
  </div>
</template>
