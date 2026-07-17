import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { Theme } from '../types'
import { useCardsStore } from './cards'
import { useAuthStore } from './auth'
import { ApiError } from '../api'

const THEMES: Theme[] = ['dark', 'light', 'rainbow', 'clouds', 'stormy']

// Global UI state: theme, text size, edit session, dirty tracking, and
// the edit-jump list. Modal/lightbox/picker state lives in stores/modal.ts;
// filter/expand state lives in stores/filters.ts.
export const useUiStore = defineStore('ui', () => {
  const theme = ref<Theme>('dark')
  const textDelta = ref(0) // px, applied to --text-delta
  const isEditing = ref(false)

  // --- Per-card unsaved-changes tracking --------------------------------------
  const dirtyIds = ref<Set<string>>(new Set())
  const hasUnsavedChanges = computed(() => dirtyIds.value.size > 0)

  // Pinia requires a new Set reference to trigger reactivity on Set mutations.
  function mutateDirty(fn: (s: Set<string>) => void) {
    const s = new Set(dirtyIds.value)
    fn(s)
    dirtyIds.value = s
  }

  function markCardDirty(id: string) {
    if (!dirtyIds.value.has(id)) mutateDirty(s => s.add(id))
  }
  function isCardDirty(id: string) {
    return dirtyIds.value.has(id)
  }
  function clearCardDirty(id: string) {
    if (!dirtyIds.value.has(id)) return
    mutateDirty(s => s.delete(id))
    const filtered = _editList.filter(e => e.cardId !== id)
    if (filtered.length !== _editList.length) {
      _setEditList(filtered)
      currentEditIndex.value = -1
    }
  }
  function clearAllDirty() {
    dirtyIds.value = new Set()
    _setEditList([])
    currentEditIndex.value = -1
  }

  // --- Edit lifecycle ---------------------------------------------------------
  const exitConfirmOpen = ref(false)
  const saving = ref(false)

  // Legend card — changes require explicit confirmation before saving.
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

  // If a write was rejected for auth reasons, drop the stale token and re-prompt.
  function handleAuthError(e: unknown): boolean {
    if (e instanceof ApiError && e.status === 401) {
      useAuthStore().logout()
      // Lazy import — ui.ts ↔ modal.ts are mutually dependent; dynamic import
      // breaks the module-level cycle. Do not restructure: keeping this in the
      // store (vs. the component) means all post-login flows stay in one place.
      import('./modal').then(({ useModalStore }) => useModalStore().openLogin(false))
      return true
    }
    return false
  }

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
  let _editList: Array<{ el: Element; cardId: string; range: Range | null }> = []
  const editCount = ref(0)
  const currentEditIndex = ref(-1)

  // Always update _editList and editCount together — never touch either directly.
  function _setEditList(next: typeof _editList) {
    _editList = next
    editCount.value = next.length
  }

  function addToEditList(cardId: string, el: Element) {
    if (_editList.some(e => e.el === el)) return
    _setEditList([..._editList, { el, cardId, range: null }])
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
    if (isEditing.value) {
      requestExit()
    } else if (useAuthStore().isAuthenticated) {
      enterEdit()
    } else {
      // Same lazy-import pattern — see handleAuthError above.
      import('./modal').then(({ useModalStore }) => useModalStore().openLogin(true))
    }
  }
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

  // ── Multi-car interrupt signal ───────────────────────────────────────────────
  // Set by ImagePicker when the interrupt fires; consumed by CardView to trigger
  // variant creation in RecipeSection. Cleared by CardView after consuming.
  const pendingMultiCarTrigger = ref<{ cardId: string; carId: string } | null>(null)
  function triggerMultiCar(cardId: string, carId: string) {
    pendingMultiCarTrigger.value = { cardId, carId }
  }
  function consumeMultiCarTrigger() {
    pendingMultiCarTrigger.value = null
  }

  return {
    theme, textDelta, isEditing, THEMES,
    dirtyIds, hasUnsavedChanges, markCardDirty, isCardDirty, clearCardDirty, clearAllDirty,
    exitConfirmOpen, saving,
    legendConfirmOpen, requestLegendConfirm, confirmLegendUpdate, cancelLegendUpdate,
    editCount, currentEditIndex, addToEditList, saveRange, setFocusedEdit, getEditAt,
    enterEdit, requestExit, toggleEdit, saveCard, saveAllDirty,
    confirmSaveAndExit, confirmDiscardAndExit, cancelExit,
    pendingMultiCarTrigger, triggerMultiCar, consumeMultiCarTrigger,
  }
})
