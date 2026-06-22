import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Livery } from '../types'
import { api } from '../api'

// Holds the catalog data and all mutations. Replaces the original app's
// DOM-as-state model: every edit updates a reactive Livery object, and save()
// persists the whole object via the API.
export const useLiveriesStore = defineStore('liveries', () => {
  const liveries = ref<Livery[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  // Per-card baseline (id -> JSON) of the last saved/loaded state, used to
  // discard unsaved edits. Saving a card refreshes its baseline so that a later
  // "Discard" only reverts cards that are still unsaved.
  let snapshots: Record<string, string> = {}

  function byId(id: string) {
    return liveries.value.find((l) => l.id === id)
  }

  async function load() {
    loading.value = true
    error.value = null
    try {
      liveries.value = await api.listLiveries()
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function save(id: string) {
    const l = byId(id)
    if (!l) return
    await api.saveLivery(l)
    snapshots[id] = JSON.stringify(l) // saved state becomes the new baseline
  }

  // Capture a baseline for every card (called when entering edit mode).
  function takeSnapshot() {
    snapshots = {}
    for (const l of liveries.value) snapshots[l.id] = JSON.stringify(l)
  }
  // Revert every card to its baseline. Cards saved during this edit session have
  // an updated baseline, so they are left at their saved state; only unsaved
  // cards roll back.
  function restoreSnapshot() {
    liveries.value = liveries.value.map((l) =>
      snapshots[l.id] ? (JSON.parse(snapshots[l.id]) as Livery) : l,
    )
  }

  function setFigure(id: string, target: 'inspiration' | 'notes', path: string) {
    const l = byId(id)
    if (!l) return
    if (target === 'inspiration') l.inspiration.figurePath = path
    else l.designNotes.figurePath = path
  }

  function toggleFavorite(id: string) {
    const l = byId(id)
    if (l) l.isFavorite = !l.isFavorite
  }

  function setLeadImage(id: string, imageId: string) {
    const l = byId(id)
    if (!l) return
    l.images.forEach((img) => (img.isLead = img.id === imageId))
  }

  // Move an image within a livery's gallery and renumber order.
  function reorderImages(id: string, fromIndex: number, toIndex: number) {
    const l = byId(id)
    if (!l) return
    const imgs = [...l.images].sort((a, b) => a.order - b.order)
    const [moved] = imgs.splice(fromIndex, 1)
    imgs.splice(toIndex, 0, moved)
    imgs.forEach((img, i) => (img.order = i))
    l.images = imgs
  }

  function addTag(id: string, value: string) {
    const l = byId(id)
    if (l && value && !l.tags.includes(value)) l.tags.push(value)
  }
  function removeTag(id: string, value: string) {
    const l = byId(id)
    if (l) l.tags = l.tags.filter((t) => t !== value)
  }
  function addCollection(id: string, value: string) {
    const l = byId(id)
    if (l && value && !l.collections.includes(value)) l.collections.push(value)
  }
  function removeCollection(id: string, value: string) {
    const l = byId(id)
    if (l) l.collections = l.collections.filter((c) => c !== value)
  }

  // All distinct chip values across the catalog (for the chip picker).
  function allTagValues() {
    return [...new Set(liveries.value.flatMap((l) => l.tags))].sort()
  }
  function allCollectionValues() {
    return [...new Set(liveries.value.flatMap((l) => l.collections))].sort()
  }

  return {
    liveries, loading, error,
    byId, load, save,
    takeSnapshot, restoreSnapshot, setFigure,
    toggleFavorite, setLeadImage, reorderImages,
    addTag, removeTag, addCollection, removeCollection,
    allTagValues, allCollectionValues,
  }
})
