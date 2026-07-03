import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useModalStore = defineStore('modal', () => {
  // ── Lightbox ────────────────────────────────────────────────────────────────
  const lightboxSrc = ref<string | null>(null)
  const lightboxOriginalSrc = ref<string | null>(null)
  const lightboxImages = ref<{ display: string; original: string }[]>([])
  const lightboxIndex = ref(0)

  function openLightbox(
    displaySrc: string,
    originalSrc?: string,
    images?: { display: string; original: string }[],
    index?: number,
  ) {
    lightboxSrc.value = displaySrc
    lightboxOriginalSrc.value = originalSrc ?? displaySrc
    lightboxImages.value = images ?? []
    lightboxIndex.value = index ?? 0
  }
  function closeLightbox() {
    lightboxSrc.value = null
    lightboxOriginalSrc.value = null
    lightboxImages.value = []
    lightboxIndex.value = 0
  }
  function navigateLightbox(dir: 1 | -1) {
    const imgs = lightboxImages.value
    if (imgs.length < 2) return
    const next = (lightboxIndex.value + dir + imgs.length) % imgs.length
    lightboxIndex.value = next
    lightboxSrc.value = imgs[next].display
    lightboxOriginalSrc.value = imgs[next].original
  }

  // ── Chip picker ─────────────────────────────────────────────────────────────
  const chipPicker = ref<{ cardId: string; type: 'tag' | 'collection' } | null>(null)
  function openChipPicker(cardId: string, type: 'tag' | 'collection') {
    chipPicker.value = { cardId, type }
  }
  function closeChipPicker() { chipPicker.value = null }

  // ── Image picker / gallery manager ──────────────────────────────────────────
  const imagePicker = ref<{ cardId: string; sectionKey?: string } | null>(null)
  function openImagePicker(cardId: string, sectionKey: string) {
    imagePicker.value = { cardId, sectionKey }
  }
  function openGalleryManager(cardId: string) { imagePicker.value = { cardId } }
  function closeImagePicker() { imagePicker.value = null }

  // ── Login modal ─────────────────────────────────────────────────────────────
  const loginOpen = ref(false)
  let loginThenEdit = false
  function openLogin(thenEdit = false) {
    loginThenEdit = thenEdit
    loginOpen.value = true
  }
  function closeLogin() { loginOpen.value = false }
  function onLoginSuccess() {
    loginOpen.value = false
    if (loginThenEdit) {
      loginThenEdit = false
      // Lazy call — avoids circular import at module level.
      import('./ui').then(({ useUiStore }) => useUiStore().enterEdit())
    }
  }

  // ── Settings modal ──────────────────────────────────────────────────────────
  const settingsOpen = ref(false)
  function openSettings() { settingsOpen.value = true }
  function closeSettings() { settingsOpen.value = false }

  // ── New card modal ──────────────────────────────────────────────────────────
  const newCardOpen = ref(false)
  function openNewCard() { newCardOpen.value = true }
  function closeNewCard() { newCardOpen.value = false }

  // ── Factoid panel ───────────────────────────────────────────────────────────
  const factoidPanelOpen = ref(false)
  function openFactoidPanel() { factoidPanelOpen.value = true }
  function closeFactoidPanel() { factoidPanelOpen.value = false }

  return {
    lightboxSrc, lightboxOriginalSrc, lightboxImages, lightboxIndex,
    openLightbox, closeLightbox, navigateLightbox,
    chipPicker, openChipPicker, closeChipPicker,
    imagePicker, openImagePicker, openGalleryManager, closeImagePicker,
    loginOpen, openLogin, closeLogin, onLoginSuccess,
    settingsOpen, openSettings, closeSettings,
    newCardOpen, openNewCard, closeNewCard,
    factoidPanelOpen, openFactoidPanel, closeFactoidPanel,
  }
})
