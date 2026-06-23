import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Card, TextSection } from '../types'
import { api } from '../api'

// Holds the catalog data and all mutations. Replaces the original app's
// DOM-as-state model: every edit updates a reactive Card object, and save()
// persists the whole object via the API.
export const useCardsStore = defineStore('cards', () => {
  const cards = ref<Card[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  // Per-card baseline (id -> JSON) of the last saved/loaded state, used to
  // discard unsaved edits. Saving a card refreshes its baseline so that a later
  // "Discard" only reverts cards that are still unsaved.
  let snapshots: Record<string, string> = {}

  function byId(id: string) {
    return cards.value.find((c) => c.id === id)
  }

  async function load() {
    loading.value = true
    error.value = null
    try {
      cards.value = await api.listCards()
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function createNewCard(fields: { name: string; subtitle: string; collections: string[] }): Promise<Card> {
    const maxCatalog = cards.value.reduce((m, c) => Math.max(m, c.catalogNumber), 0)
    const nextNum = maxCatalog + 1
    const newCard: Card = {
      id: String(nextNum),
      catalogNumber: nextNum,
      name: fields.name,
      subtitle: fields.subtitle,
      isFavorite: false,
      isLegend: false,
      collections: fields.collections,
      tags: [],
      images: [],
      sections: [],
    }
    const created = await api.createCard(newCard)
    cards.value.push(created)
    return created
  }

  async function save(id: string) {
    const c = byId(id)
    if (!c) return
    await api.saveCard(c)
    snapshots[id] = JSON.stringify(c) // saved state becomes the new baseline
  }

  // Capture a baseline for every card (called when entering edit mode).
  function takeSnapshot() {
    snapshots = {}
    for (const c of cards.value) snapshots[c.id] = JSON.stringify(c)
  }
  // Revert every card to its baseline. Cards saved during this edit session have
  // an updated baseline, so they are left at their saved state; only unsaved
  // cards roll back. Images added since the last save are deleted from disk.
  async function restoreSnapshot() {
    const orphans: string[] = []
    for (const c of cards.value) {
      const snap = snapshots[c.id]
      if (!snap) continue
      const savedPaths = new Set(
        (JSON.parse(snap) as Card).images.flatMap((i) =>
          [i.path, i.thumbPath, i.stagePath].filter(Boolean) as string[]
        )
      )
      for (const img of c.images) {
        for (const p of [img.path, img.thumbPath, img.stagePath]) {
          if (p && !savedPaths.has(p)) orphans.push(p)
        }
      }
    }
    if (orphans.length) await api.deleteImages(orphans).catch(() => {})
    cards.value = cards.value.map((c) =>
      snapshots[c.id] ? (JSON.parse(snapshots[c.id]) as Card) : c,
    )
  }

  // Set the figure image on a text section (identified by its key).
  function setFigure(id: string, sectionKey: string, path: string) {
    const c = byId(id)
    const section = c?.sections.find(
      (s): s is TextSection => s.type === 'text' && s.key === sectionKey,
    )
    if (section) section.figurePath = path
  }

  function toggleFavorite(id: string) {
    const c = byId(id)
    if (c) c.isFavorite = !c.isFavorite
  }

  // Make an image the lead by moving it to order 0 (lead === order 0), then
  // renumber the rest.
  function setLeadImage(id: string, imageId: string) {
    const c = byId(id)
    if (!c) return
    const imgs = [...c.images].sort((a, b) => a.order - b.order)
    const pos = imgs.findIndex((img) => img.id === imageId)
    if (pos > 0) {
      const [moved] = imgs.splice(pos, 1)
      imgs.unshift(moved)
    }
    imgs.forEach((img, i) => (img.order = i))
    c.images = imgs
  }

  // Move an image within a card's gallery and renumber order.
  function reorderImages(id: string, fromIndex: number, toIndex: number) {
    const c = byId(id)
    if (!c) return
    const imgs = [...c.images].sort((a, b) => a.order - b.order)
    const [moved] = imgs.splice(fromIndex, 1)
    imgs.splice(toIndex, 0, moved)
    imgs.forEach((img, i) => (img.order = i))
    c.images = imgs
  }

  function removeImage(cardId: string, imageId: string) {
    const c = byId(cardId)
    if (!c) return
    c.images = c.images.filter((i) => i.id !== imageId)
  }

  function toggleImageIncluded(id: string, imageId: string) {
    const c = byId(id)
    if (!c) return
    const img = c.images.find((i) => i.id === imageId)
    if (!img) return
    img.included = img.included === false ? true : false
  }

  function addImageToPool(
    cardId: string,
    path: string,
    thumbPath?: string,
    stagePath?: string,
  ) {
    const c = byId(cardId)
    if (!c) return
    const maxOrder = c.images.reduce((m, i) => Math.max(m, i.order), -1)
    c.images.push({ id: `${cardId}-${Date.now()}`, path, thumbPath, stagePath, order: maxOrder + 1, included: true })
  }

  function addTag(id: string, value: string) {
    const c = byId(id)
    if (c && value && !c.tags.includes(value)) c.tags.push(value)
  }
  function removeTag(id: string, value: string) {
    const c = byId(id)
    if (c) c.tags = c.tags.filter((t) => t !== value)
  }
  function addCollection(id: string, value: string) {
    const c = byId(id)
    if (c && value && !c.collections.includes(value)) c.collections.push(value)
  }
  function removeCollection(id: string, value: string) {
    const c = byId(id)
    if (c) c.collections = c.collections.filter((x) => x !== value)
  }

  // All distinct chip values across the catalog (for the chip picker).
  function allTagValues() {
    return [...new Set(cards.value.flatMap((c) => c.tags))].sort()
  }
  function allCollectionValues() {
    return [...new Set(cards.value.flatMap((c) => c.collections))].sort()
  }

  // Distinct sections across the catalog, in first-seen order — drives the
  // generic section filter in the side-bug menu.
  function allSectionKeys() {
    const seen = new Map<string, string>() // key -> label
    for (const c of cards.value) {
      for (const s of c.sections) if (!seen.has(s.key)) seen.set(s.key, s.label)
    }
    return [...seen].map(([key, label]) => ({ key, label }))
  }

  return {
    cards, loading, error,
    byId, load, save, createNewCard,
    takeSnapshot, restoreSnapshot, setFigure,
    toggleFavorite, setLeadImage, reorderImages,
    removeImage, toggleImageIncluded, addImageToPool,
    addTag, removeTag, addCollection, removeCollection,
    allTagValues, allCollectionValues, allSectionKeys,
  }
})
