<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import { useLiveriesStore } from './stores/liveries'
import { useUiStore } from './stores/ui'
import SideBug from './components/SideBug.vue'
import Filters from './components/Filters.vue'
import LiveryCard from './components/LiveryCard.vue'
import EditBar from './components/EditBar.vue'
import Lightbox from './components/Lightbox.vue'
import ChipPicker from './components/ChipPicker.vue'
import ImagePicker from './components/ImagePicker.vue'
import ExitConfirmModal from './components/ExitConfirmModal.vue'
import CustomTip from './components/CustomTip.vue'

const store = useLiveriesStore()
const ui = useUiStore()

function onKey(e: KeyboardEvent) {
  if (e.key !== 'Escape') return
  ui.closeLightbox()
  ui.closeChipPicker()
  ui.closeImagePicker()
  if (ui.exitConfirmOpen) ui.cancelExit()
}

onMounted(() => {
  store.load()
  document.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => document.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="page-head">
    <p class="eyebrow">Festival Garage / Personal Collection</p>
    <h1>Livery Catalog</h1>
    <p>A running archive of liveries pulled from the garage — one card per design, every angle on file, share codes to follow once they're locked in.</p>
    <span class="draft-tag">Draft — layout preview, not final</span>
  </div>

  <SideBug>
    <template #menu><Filters /></template>
  </SideBug>
  <EditBar />

  <div class="catalog">
    <p v-if="store.loading">Loading…</p>
    <p v-else-if="store.error">Failed to load: {{ store.error }}</p>
    <template v-else>
      <LiveryCard
        v-for="l in store.liveries"
        :key="l.id"
        :livery="l"
        v-show="ui.isLiveryVisible(l.collections, l.isFavorite)"
      />
    </template>
  </div>

  <Lightbox />
  <ChipPicker />
  <ImagePicker />
  <ExitConfirmModal />
  <CustomTip />
</template>
