<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, ref, provide } from 'vue'
import { DynamicScroller, DynamicScrollerItem } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'
import { useScrollGuard } from './composables/useScrollGuard'
import { useCardsStore } from './stores/cards'
import { useUiStore } from './stores/ui'
import { useFilterStore } from './stores/filters'
import { useModalStore } from './stores/modal'
import { useThemeStore } from './stores/theme'
import SideBug from './components/SideBug.vue'
import Filters from './components/Filters.vue'
import CardView from './components/CardView.vue'
import CardShell from './components/CardShell.vue'
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
import EditCardModal from './components/EditCardModal.vue'
import ImageMigrationModal from './components/ImageMigrationModal.vue'
import AdminPanel from './components/AdminPanel.vue'
import ArchiveCardModal from './components/ArchiveCardModal.vue'

useScrollGuard()

const store = useCardsStore()
const ui = useUiStore()
const filters = useFilterStore()
const modal = useModalStore()
const theme = useThemeStore()

const scrollerRef = ref<{ scrollToItem: (idx: number) => void } | null>(null)

const visibleCards = computed(() =>
  store.cards.filter(c => c.isLegend ? ui.isEditing : filters.isCardVisible(c))
)

function scrollToCardId(id: string) {
  const idx = visibleCards.value.findIndex(c => c.id === id)
  if (idx < 0) return
  scrollerRef.value?.scrollToItem(idx)
  // Estimated position may be off for unmeasured cards — refine once rendered
  requestAnimationFrame(() => requestAnimationFrame(() => {
    const el = document.getElementById(`card-${id}`)
    if (el) window.scrollTo({ top: el.getBoundingClientRect().top + window.scrollY - 20 })
  }))
}
provide('scrollToCardId', scrollToCardId)

function onKey(e: KeyboardEvent) {
  if (e.key !== 'Escape') return
  if (!modal.closeTopModal() && ui.exitConfirmOpen) ui.cancelExit()
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
    <DynamicScroller
      v-else
      ref="scrollerRef"
      :items="visibleCards"
      :min-item-size="1200"
      :buffer="800"
      key-field="id"
      :page-mode="true"
    >
      <template #default="{ item, active }">
        <DynamicScrollerItem
          :item="item"
          :active="active"
          :size-dependencies="[item.sections, item.images]"
        >
          <div class="card-gap">
            <CardShell :key="item.id">
              <CardView :card="item" />
            </CardShell>
          </div>
        </DynamicScrollerItem>
      </template>
    </DynamicScroller>
  </div>

  <footer class="catalog-credits">
    <p>Car data sourced from <a href="https://forza.fandom.com" target="_blank" rel="noopener">Forza Wiki</a> and <a href="https://www.forzamotorsport.net" target="_blank" rel="noopener">Forza Motorsport</a>. Fan project — not affiliated with Microsoft or Playground Games.</p>
  </footer>

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
  <EditCardModal v-if="modal.promotedCard" :card="modal.promotedCard" @close="modal.closePromotedCard()" />
  <ImageMigrationModal />
  <AdminPanel />
  <ArchiveCardModal />
  <CustomTip />
</template>
