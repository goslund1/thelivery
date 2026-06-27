import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { Theme } from '../types'
import { useCardsStore } from './cards'
import { useAuthStore } from './auth'
import { ApiError } from '../api'

const THEMES: Theme[] = ['dark', 'light', 'rainbow', 'clouds', 'stormy']

// Global UI state: theme, text size, edit mode, expand/collapse, filters, and
// which modal/lightbox is open. Replaces the original app's class-on-<html> and
// class-on-<body> state juggling.
export const useUiStore = defineStore('ui', () => {
  const theme = ref<Theme>('dark')
  const textDelta = ref(0) // px, applied to --text-delta
  const isEditing = ref(false)

  // --- Per-card unsaved-changes tracking --------------------------------------
  // The set of card ids edited since their last save. Each card has its own
  // save button; the exit prompt fires only if any card is still dirty.
  const dirtyIds = ref<Set<string>>(new Set())
  const hasUnsavedChanges = computed(() => dirtyIds.value.size > 0)
  function markCardDirty(id: string) {
    if (dirtyIds.value.has(id)) return
    const s = new Set(dirtyIds.value)
    s.add(id)
    dirtyIds.value = s
  }
  function isCardDirty(id: string) {
    return dirtyIds.value.has(id)
  }
  function clearCardDirty(id: string) {
    if (!dirtyIds.value.has(id)) return
    const s = new Set(dirtyIds.value)
    s.delete(id)
    dirtyIds.value = s
    const before = _editList.length
    _editList.splice(0, _editList.length, ..._editList.filter(e => e.cardId !== id))
    if (_editList.length !== before) {
      editCount.value = _editList.length
      currentEditIndex.value = -1
    }
  }
  function clearAllDirty() {
    dirtyIds.value = new Set()
    _editList.splice(0)
    editCount.value = 0
    currentEditIndex.value = -1
  }

  // --- Expand/collapse state --------------------------------------------------
  // Per the original: the section checkboxes EXPAND/COLLAPSE all sections of a
  // type (they don't hide them), and expand-all flips everything together.
  const allExpanded = ref(false)
  // Keyed by section key (e.g. "inspiration", "notes", "recipe"); populated
  // dynamically from the cards' sections. Missing key = collapsed.
  const sectionExpanded = ref<Record<string, boolean>>({})
  const upgradesExpanded = ref(false) // the "Upgrades Installed" sub-list

  function toggleAll() {
    allExpanded.value = !allExpanded.value
    const v = allExpanded.value
    for (const { key } of useCardsStore().allSectionKeys()) sectionExpanded.value[key] = v
    upgradesExpanded.value = v
  }
  function setSectionExpanded(key: string, v: boolean) {
    sectionExpanded.value[key] = v
  }

  // --- Card filters -----------------------------------------------------------
  const favoritesOnly = ref(false)
  const disabledCollections = ref<Set<string>>(new Set()) // collections turned OFF

  // A card is visible if it passes the favorites + collection filters.
  // Mirrors the original recalcCardVisibility logic.
  function isCardVisible(collections: string[], isFavorite: boolean) {
    if (favoritesOnly.value && !isFavorite) return false
    if (!collections.length) return true
    return collections.some((c) => !disabledCollections.value.has(c))
  }
  function toggleCollection(name: string) {
    const s = new Set(disabledCollections.value)
    s.has(name) ? s.delete(name) : s.add(name)
    disabledCollections.value = s
  }

  // Edit lifecycle
  const exitConfirmOpen = ref(false)
  const saving = ref(false)

  // Legend card (catalog 000) — changes require explicit confirmation before saving
  const legendConfirmOpen = ref(false)
  let _legendConfirmResolve: ((confirmed: boolean) => void) | null = null
  function requestLegendConfirm(): Promise<boolean> {
    return new Promise(resolve => {
      _legendConfirmResolve = resolve
      legendConfirmOpen.value = true
    })
  }
  function confirmLegendUpdate() {
    legendConfirmOpen.value = false
    _legendConfirmResolve?.(true)
    _legendConfirmResolve = null
  }
  function cancelLegendUpdate() {
    legendConfirmOpen.value = false
    _legendConfirmResolve?.(false)
    _legendConfirmResolve = null
  }

  // Login modal
  const loginOpen = ref(false)
  let loginThenEdit = false // whether to enter edit mode after a successful login
  function openLogin(thenEdit = false) {
    loginThenEdit = thenEdit
    loginOpen.value = true
  }
  function closeLogin() {
    loginOpen.value = false
  }

  const settingsOpen = ref(false)
  function openSettings() { settingsOpen.value = true }
  function closeSettings() { settingsOpen.value = false }
  function onLoginSuccess() {
    loginOpen.value = false
    if (loginThenEdit) {
      loginThenEdit = false
      enterEdit()
    }
  }
  // If a write was rejected for auth reasons, drop the stale token and re-prompt.
  function handleAuthError(e: unknown): boolean {
    if (e instanceof ApiError && e.status === 401) {
      useAuthStore().logout()
      openLogin(false)
      return true
    }
    return false
  }

  // Lightbox — displaySrc is what's shown (may be stage JPEG); originalSrc is always the full-res original for download.
  // images is the full reel for arrow navigation; index tracks position.
  const lightboxSrc = ref<string | null>(null)
  const lightboxOriginalSrc = ref<string | null>(null)
  const lightboxImages = ref<{ display: string; original: string }[]>([])
  const lightboxIndex = ref(0)

  // New card modal
  const newCardOpen = ref(false)
  function openNewCard() { newCardOpen.value = true }
  function closeNewCard() { newCardOpen.value = false }

  // Chip picker (add a tag/collection to a card)
  const chipPicker = ref<{ cardId: string; type: 'tag' | 'collection' } | null>(null)
  // Image picker: pick mode (sectionKey set) or manage mode (sectionKey absent).
  const imagePicker = ref<{ cardId: string; sectionKey?: string } | null>(null)

  // Apply theme + text size to <html> reactively (CSS variables drive the rest).
  watch(theme, (t) => document.documentElement.setAttribute('data-theme', t), { immediate: true })
  watch(
    textDelta,
    (d) => document.documentElement.style.setProperty('--text-delta', `${d}px`),
    { immediate: true },
  )
  watch(isEditing, (on) => document.body.classList.toggle('editing-mode', on))

  function onBeforeUnload(e: BeforeUnloadEvent) { e.preventDefault() }
  watch(hasUnsavedChanges, (dirty) => {
    if (dirty) window.addEventListener('beforeunload', onBeforeUnload)
    else window.removeEventListener('beforeunload', onBeforeUnload)
  })

  // Flat ordered list of every contenteditable that received an input during this
  // edit session, plus each field's last cursor position. Plain JS — DOM refs
  // must NOT enter Vue's reactive system (Vue proxying breaks Range objects).
  const _editList: Array<{ el: Element; cardId: string; range: Range | null }> = []
  const editCount = ref(0)
  const currentEditIndex = ref(-1) // index in _editList of the currently-focused entry

  function addToEditList(cardId: string, el: Element) {
    if (_editList.some(e => e.el === el)) return
    _editList.push({ el, cardId, range: null })
    editCount.value = _editList.length
  }
  function saveRange(el: Element, range: Range | null) {
    const entry = _editList.find(e => e.el === el)
    if (entry) entry.range = range
  }
  function setFocusedEdit(el: Element) {
    currentEditIndex.value = _editList.findIndex(e => e.el === el)
  }
  function getEditAt(idx: number): { el: Element; cardId: string; range: Range | null } | null {
    return _editList[idx] ?? null
  }

  // --- Edit lifecycle ---------------------------------------------------------
  function enterEdit() {
    useCardsStore().takeSnapshot()
    clearAllDirty()
    isEditing.value = true
  }
  function requestExit() {
    if (hasUnsavedChanges.value) exitConfirmOpen.value = true
    else isEditing.value = false
  }
  // Entering edit mode requires a login; clicking edit while signed out prompts
  // for credentials, then enters edit on success.
  function toggleEdit() {
    if (isEditing.value) {
      requestExit()
    } else if (useAuthStore().isAuthenticated) {
      enterEdit()
    } else {
      openLogin(true)
    }
  }
  // Save a single card and clear its dirty flag.
  async function saveCard(id: string) {
    saving.value = true
    try {
      await useCardsStore().save(id)
      clearCardDirty(id)
    } catch (e) {
      if (!handleAuthError(e)) throw e
    } finally {
      saving.value = false
    }
  }
  // Save every card that still has unsaved changes.
  // If the legend card (000) is dirty, pause and confirm before saving it.
  async function saveAllDirty() {
    const cardsStore = useCardsStore()
    const legendId = cardsStore.cards.find(c => c.isLegend)?.id
    const allIds = [...dirtyIds.value]
    const regularIds = legendId ? allIds.filter(id => id !== legendId) : allIds
    const legendIsDirty = !!legendId && allIds.includes(legendId)

    saving.value = true
    try {
      await Promise.all(regularIds.map(id => cardsStore.save(id)))
      for (const id of regularIds) clearCardDirty(id)
    } catch (e) {
      if (!handleAuthError(e)) throw e
    } finally {
      saving.value = false
    }

    if (legendIsDirty && legendId) {
      const confirmed = await requestLegendConfirm()
      if (confirmed) {
        saving.value = true
        try {
          await cardsStore.save(legendId)
          clearCardDirty(legendId)
        } catch (e) {
          if (!handleAuthError(e)) throw e
        } finally {
          saving.value = false
        }
      } else {
        clearCardDirty(legendId)
      }
    }
  }
  async function confirmSaveAndExit() {
    exitConfirmOpen.value = false
    await saveAllDirty()
    isEditing.value = false
  }
  async function confirmDiscardAndExit() {
    await useCardsStore().restoreSnapshot()
    clearAllDirty()
    exitConfirmOpen.value = false
    isEditing.value = false
  }
  function cancelExit() {
    exitConfirmOpen.value = false
  }
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
  function navigateLightbox(dir: 1 | -1) {
    const imgs = lightboxImages.value
    if (imgs.length < 2) return
    const next = (lightboxIndex.value + dir + imgs.length) % imgs.length
    lightboxIndex.value = next
    lightboxSrc.value = imgs[next].display
    lightboxOriginalSrc.value = imgs[next].original
  }
  function closeLightbox() {
    lightboxSrc.value = null
    lightboxOriginalSrc.value = null
    lightboxImages.value = []
    lightboxIndex.value = 0
  }
  function openChipPicker(cardId: string, type: 'tag' | 'collection') {
    chipPicker.value = { cardId, type }
  }
  function closeChipPicker() {
    chipPicker.value = null
  }
  function openImagePicker(cardId: string, sectionKey: string) {
    imagePicker.value = { cardId, sectionKey }
  }
  function openGalleryManager(cardId: string) {
    imagePicker.value = { cardId }
  }
  function closeImagePicker() {
    imagePicker.value = null
  }

  return {
    theme, textDelta, isEditing,
    dirtyIds, hasUnsavedChanges, markCardDirty, isCardDirty, clearCardDirty, clearAllDirty,
    allExpanded, sectionExpanded, upgradesExpanded, toggleAll, setSectionExpanded,
    favoritesOnly, disabledCollections,
    isCardVisible, toggleCollection,
    exitConfirmOpen, saving,
    legendConfirmOpen, confirmLegendUpdate, cancelLegendUpdate,
    loginOpen, openLogin, closeLogin, onLoginSuccess,
    settingsOpen, openSettings, closeSettings,
    newCardOpen, openNewCard, closeNewCard,
    lightboxSrc, lightboxOriginalSrc, lightboxImages, lightboxIndex, chipPicker, imagePicker,
    THEMES,
    editCount, currentEditIndex, addToEditList, saveRange, setFocusedEdit, getEditAt,
    enterEdit, requestExit, toggleEdit, saveCard, saveAllDirty,
    confirmSaveAndExit, confirmDiscardAndExit, cancelExit,
    openLightbox, navigateLightbox, closeLightbox,
    openChipPicker, closeChipPicker,
    openImagePicker, openGalleryManager, closeImagePicker,
  }
})
