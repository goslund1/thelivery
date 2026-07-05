<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import { useCardsStore } from './stores/cards'
import { useUiStore } from './stores/ui'
import { useFilterStore } from './stores/filters'
import { useModalStore } from './stores/modal'
import { useThemeStore } from './stores/theme'
import SideBug from './components/SideBug.vue'
import Filters from './components/Filters.vue'
import CardView from './components/CardView.vue'
import EditBar from './components/EditBar.vue'
import Lightbox from './components/Lightbox.vue'
import ChipPicker from './components/ChipPicker.vue'
import ImagePicker from './components/ImagePicker.vue'
import ExitConfirmModal from './components/ExitConfirmModal.vue'
import LoginModal from './components/LoginModal.vue'
import UserSettingsModal from './components/UserSettingsModal.vue'
import CustomTip from './components/CustomTip.vue'
import NewCardModal from './components/NewCardModal.vue'
import LegendConfirmModal from './components/LegendConfirmModal.vue'
import FactoidPanel from './components/FactoidPanel.vue'
import CardHistoryModal from './components/CardHistoryModal.vue'
import SuggestionViewer from './components/SuggestionViewer.vue'

const store = useCardsStore()
const ui = useUiStore()
const filters = useFilterStore()
const modal = useModalStore()
const theme = useThemeStore()

function onKey(e: KeyboardEvent) {
  if (e.key !== 'Escape') return
  modal.closeLightbox()
  modal.closeChipPicker()
  modal.closeImagePicker()
  if (modal.loginOpen) modal.closeLogin()
  if (modal.settingsOpen) modal.closeSettings()
  modal.closeNewCard()
  if (ui.exitConfirmOpen) ui.cancelExit()
  if (modal.factoidPanelOpen) modal.closeFactoidPanel()
}

function checkIgnition() {
  if (window.location.pathname === '/ignition' || window.location.hash === '#ignition') {
    window.history.replaceState(null, '', '/')
    modal.openLogin(true)
  }
}

onMounted(() => {
  store.load()
  theme.load()
  document.addEventListener('keydown', onKey)
  window.addEventListener('hashchange', checkIgnition)
  checkIgnition()
})
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('hashchange', checkIgnition)
})
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
      <CardView
        v-for="c in store.cards"
        :key="c.id"
        :card="c"
        v-show="c.isLegend ? ui.isEditing : filters.isCardVisible(c.collections, c.isFavorite)"
      />
    </template>
  </div>

  <Lightbox />
  <ChipPicker />
  <ImagePicker />
  <ExitConfirmModal />
  <LoginModal />
  <UserSettingsModal />
  <NewCardModal />
  <LegendConfirmModal />
  <FactoidPanel />
  <CardHistoryModal v-if="modal.historyCardId" :card-id="modal.historyCardId" />
  <SuggestionViewer v-if="modal.suggestionViewerOpen" @close="modal.closeSuggestionViewer()" />
  <CustomTip />
</template>
