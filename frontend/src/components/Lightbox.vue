<script setup lang="ts">
import { useUiStore } from '../stores/ui'
const ui = useUiStore()
</script>

<template>
  <div class="lightbox" :class="{ open: !!ui.lightboxSrc }" @click.self="ui.closeLightbox()">
    <button class="lightbox-close" aria-label="Close" @click="ui.closeLightbox()">×</button>
    <img v-if="ui.lightboxSrc" :src="ui.lightboxSrc" alt="" />
  </div>
</template>
