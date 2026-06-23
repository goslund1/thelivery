import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { Theme } from '../types'
import { useCardsStore } from './cards'

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
  }
  function clearAllDirty() {
    dirtyIds.value = new Set()
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

  // Lightbox
  const lightboxSrc = ref<string | null>(null)

  // New card modal
  const newCardOpen = ref(false)
  function openNewCard() { newCardOpen.value = true }
  function closeNewCard() { newCardOpen.value = false }

  // Chip picker (add a tag/collection to a card)
  const chipPicker = ref<{ cardId: string; type: 'tag' | 'collection' } | null>(null)
  // Image picker (set a section's figure image, or upload). `sectionKey` says where.
  const imagePicker = ref<{ cardId: string; sectionKey: string } | null>(null)

  // Apply theme + text size to <html> reactively (CSS variables drive the rest).
  watch(theme, (t) => document.documentElement.setAttribute('data-theme', t), { immediate: true })
  watch(
    textDelta,
    (d) => document.documentElement.style.setProperty('--text-delta', `${d}px`),
    { immediate: true },
  )
  watch(isEditing, (on) => document.body.classList.toggle('editing-mode', on))

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
  function toggleEdit() {
    if (isEditing.value) requestExit()
    else enterEdit()
  }
  // Save a single card and clear its dirty flag.
  async function saveCard(id: string) {
    saving.value = true
    try {
      await useCardsStore().save(id)
      clearCardDirty(id)
    } finally {
      saving.value = false
    }
  }
  // Save every card that still has unsaved changes.
  async function saveAllDirty() {
    saving.value = true
    try {
      const ids = [...dirtyIds.value]
      await Promise.all(ids.map((id) => useCardsStore().save(id)))
      clearAllDirty()
    } finally {
      saving.value = false
    }
  }
  async function confirmSaveAndExit() {
    await saveAllDirty()
    exitConfirmOpen.value = false
    isEditing.value = false
  }
  function confirmDiscardAndExit() {
    useCardsStore().restoreSnapshot()
    clearAllDirty()
    exitConfirmOpen.value = false
    isEditing.value = false
  }
  function cancelExit() {
    exitConfirmOpen.value = false
  }
  function openLightbox(src: string) {
    lightboxSrc.value = src
  }
  function closeLightbox() {
    lightboxSrc.value = null
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
    newCardOpen, openNewCard, closeNewCard,
    lightboxSrc, chipPicker, imagePicker,
    THEMES,
    enterEdit, requestExit, toggleEdit, saveCard, saveAllDirty,
    confirmSaveAndExit, confirmDiscardAndExit, cancelExit,
    openLightbox, closeLightbox,
    openChipPicker, closeChipPicker,
    openImagePicker, closeImagePicker,
  }
})
