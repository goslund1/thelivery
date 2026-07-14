import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Card } from '../types'

export interface PoolImage {
  id?: number
  path: string
  thumbPath?: string
  stagePath?: string
}

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
  const imagePicker = ref<{
    cardId: string | null
    initialCarId?: string | null
    sectionKey?: string
    onPick?: (path: string, img?: PoolImage) => void
    // Getter for the pending pool in new-card creation mode. Using a function avoids
    // Vue's nested-ref auto-unwrapping, while still letting the gallery computed track
    // the underlying reactive ref when the getter is called inside the computed.
    getPool?: () => PoolImage[]
    // Called by the Photo Manager overlay when the first upload needs a card_id but none exists yet.
    // Returns the newly created card's id. Receives the session carId so the card can be pre-tagged.
    ensureCard?: (carId: string | null) => Promise<string>
  } | null>(null)
  function openImagePicker(cardId: string, sectionKey: string) {
    imagePicker.value = { cardId, sectionKey }
  }
  function openGalleryManager(cardId: string, initialCarId?: string | null) { imagePicker.value = { cardId, initialCarId } }
  // Open the Photo Manager in manage mode for a card that may not exist yet.
  function openManagePhotos(cardId: string | null, ensureCard?: (carId: string | null) => Promise<string>) {
    imagePicker.value = { cardId, ensureCard }
  }
  // Figure-picker for new-card creation: getPool returns the modal's pendingPool contents,
  // onPick is called with the chosen path. For edit-mode figure picks, omit getPool.
  function openFigurePicker(
    cardId: string | null,
    getPool: (() => PoolImage[]) | null,
    onPick: (path: string, img?: PoolImage) => void,
    ensureCard?: (carId: string | null) => Promise<string>,
  ) {
    imagePicker.value = { cardId, sectionKey: 'figure', onPick, getPool: getPool ?? undefined, ensureCard }
  }
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
  const newCardOpenCount = ref(0)
  function openNewCard() { newCardOpen.value = true; newCardOpenCount.value++ }
  function closeNewCard() { newCardOpen.value = false }

  // ── Factoid panel ───────────────────────────────────────────────────────────
  const factoidPanelOpen = ref(false)
  function openFactoidPanel() { factoidPanelOpen.value = true }
  function closeFactoidPanel() { factoidPanelOpen.value = false }

  // ── Card history modal ───────────────────────────────────────────────────────
  const historyCardId = ref<string | null>(null)
  function openHistory(cardId: string) { historyCardId.value = cardId }
  function closeHistory() { historyCardId.value = null }

  // ── Suggestion viewer ────────────────────────────────────────────────────────
  const suggestionViewerOpen = ref(false)
  const pendingSuggestionCount = ref(0)
  function openSuggestionViewer() { suggestionViewerOpen.value = true }
  function closeSuggestionViewer() { suggestionViewerOpen.value = false }

  // ── Promoted card editor ─────────────────────────────────────────────────────
  const promotedCard = ref<Card | null>(null)
  function openPromotedCard(card: Card) { promotedCard.value = card }
  function closePromotedCard() { promotedCard.value = null }

  // ── Image migration tool ─────────────────────────────────────────────────────
  const imageMigrationOpen = ref(false)
  function openImageMigration() { imageMigrationOpen.value = true }
  function closeImageMigration() { imageMigrationOpen.value = false }

  // ── Admin panel ──────────────────────────────────────────────────────────────
  const adminPanelOpen = ref(false)
  function openAdminPanel() { adminPanelOpen.value = true }
  function closeAdminPanel() { adminPanelOpen.value = false }

  // ── Theme builder ─────────────────────────────────────────────────────────────
  const themeBuilderOpen = ref(false)
  function openThemeBuilder() { themeBuilderOpen.value = true }
  function closeThemeBuilder() { themeBuilderOpen.value = false }

  // ── Share modal ──────────────────────────────────────────────────────────
  const shareCardId = ref<string | null>(null)
  function openShare(cardId: string) { shareCardId.value = cardId }
  function closeShare() { shareCardId.value = null }

  // ── Archive card confirm ─────────────────────────────────────────────────────
  const archiveCardPending = ref(false)
  const archiveCardName = ref('')
  let _archiveResolve: ((confirmed: boolean) => void) | null = null

  function promptArchiveCard(name: string): Promise<boolean> {
    archiveCardName.value = name
    archiveCardPending.value = true
    return new Promise(resolve => { _archiveResolve = resolve })
  }
  function confirmArchiveCard() {
    archiveCardPending.value = false
    _archiveResolve?.(true)
    _archiveResolve = null
  }
  function cancelArchiveCard() {
    archiveCardPending.value = false
    _archiveResolve?.(false)
    _archiveResolve = null
  }

  // Close whichever modal is frontmost. Returns true if anything was closed.
  // Priority order mirrors visual z-order: innermost/topmost first.
  function closeTopModal(): boolean {
    if (promotedCard.value)          { closePromotedCard();     return true }
    if (lightboxSrc.value)           { closeLightbox();         return true }
    if (chipPicker.value)            { closeChipPicker();       return true }
    if (imagePicker.value)           { closeImagePicker();      return true }
    if (historyCardId.value)         { closeHistory();          return true }
    if (suggestionViewerOpen.value)  { closeSuggestionViewer(); return true }
    if (loginOpen.value)             { closeLogin();            return true }
    if (adminPanelOpen.value)        { closeAdminPanel();       return true }
    if (themeBuilderOpen.value)      { closeThemeBuilder();     return true }
    if (settingsOpen.value)          { closeSettings();         return true }
    if (newCardOpen.value)           { closeNewCard();          return true }
    if (shareCardId.value)            { closeShare();            return true }
    if (factoidPanelOpen.value)      { closeFactoidPanel();     return true }
    return false
  }

  return {
    lightboxSrc, lightboxOriginalSrc, lightboxImages, lightboxIndex,
    openLightbox, closeLightbox, navigateLightbox,
    chipPicker, openChipPicker, closeChipPicker,
    imagePicker, openImagePicker, openGalleryManager, openManagePhotos, openFigurePicker, closeImagePicker,
    loginOpen, openLogin, closeLogin, onLoginSuccess,
    settingsOpen, openSettings, closeSettings,
    newCardOpen, newCardOpenCount, openNewCard, closeNewCard,
    factoidPanelOpen, openFactoidPanel, closeFactoidPanel,
    historyCardId, openHistory, closeHistory,
    suggestionViewerOpen, pendingSuggestionCount, openSuggestionViewer, closeSuggestionViewer,
    promotedCard, openPromotedCard, closePromotedCard,
    imageMigrationOpen, openImageMigration, closeImageMigration,
    adminPanelOpen, openAdminPanel, closeAdminPanel,
    themeBuilderOpen, openThemeBuilder, closeThemeBuilder,
    shareCardId, openShare, closeShare,
    archiveCardPending, archiveCardName, promptArchiveCard, confirmArchiveCard, cancelArchiveCard,
    closeTopModal,
  }
})
