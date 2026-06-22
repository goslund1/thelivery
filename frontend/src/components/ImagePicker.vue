<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { useLiveriesStore } from '../stores/liveries'
import { api } from '../api'

const ui = useUiStore()
const store = useLiveriesStore()

const ctx = computed(() => ui.imagePicker)
const livery = computed(() => (ctx.value ? store.byId(ctx.value.liveryId) : undefined))
const gallery = computed(() => [...(livery.value?.images ?? [])].sort((a, b) => a.order - b.order))

function pick(path: string) {
  const c = ctx.value
  if (!c) return
  store.setFigure(c.liveryId, c.target, path)
  ui.markCardDirty(c.liveryId)
  ui.closeImagePicker()
}

async function onUpload(e: Event) {
  const c = ctx.value
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!c || !file) return
  const { path } = await api.uploadImage(file)
  store.setFigure(c.liveryId, c.target, path)
  ui.markCardDirty(c.liveryId)
  ui.closeImagePicker()
}
</script>

<template>
  <div class="image-picker" :class="{ open: !!ctx }" @click.self="ui.closeImagePicker()">
    <div class="image-picker-panel">
      <div class="image-picker-head">
        <span>Choose a feature image</span>
        <button class="image-picker-close" aria-label="Close" @click="ui.closeImagePicker()">×</button>
      </div>
      <div class="image-picker-grid">
        <img v-for="img in gallery" :key="img.id" :src="img.path" @click="pick(img.path)" />
      </div>
      <label class="image-picker-upload">
        Or upload a new image…
        <input type="file" accept="image/*" @change="onUpload" />
      </label>
    </div>
  </div>
</template>
