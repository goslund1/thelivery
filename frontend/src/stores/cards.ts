import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Card, TextSection, UpgradeCategory, Adjustment } from '../types'
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

  function ensureSections(card: Card): Card {
    const keys = new Set(card.sections.map(s => s.key))
    const missing: Card['sections'] = []
    if (!keys.has('inspiration'))
      missing.push({ type: 'text', key: 'inspiration', label: 'Inspiration', body: '' })
    if (!keys.has('notes'))
      missing.push({ type: 'text', key: 'notes', label: 'Design Notes', body: '' })
    if (!keys.has('recipe'))
      missing.push({ type: 'forza_recipe', key: 'recipe', label: 'Tune / Build Parts', tuneName: '', shareCode: '', coreSpecs: {}, upgrades: [], adjustments: [] })
    if (missing.length === 0) return card
    return { ...card, sections: [...card.sections, ...missing] }
  }

  async function load() {
    loading.value = true
    error.value = null
    try {
      cards.value = (await api.listCards()).map(ensureSections)
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function createNewCard(fields: {
    name: string
    subtitle: string
    collections: string[]
    tags?: string[]
    inspirationBody?: string
    inspirationFigurePath?: string
    notesBody?: string
    notesFigurePath?: string
    tuneName?: string
    shareCode?: string
    coreSpecs?: Record<string, string>
    upgrades?: UpgradeCategory[]
    adjustments?: Adjustment[]
  }): Promise<Card> {
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
      tags: fields.tags ?? [],
      images: [],
      sections: [
        { type: 'text', key: 'inspiration', label: 'Inspiration', body: fields.inspirationBody ?? '', figurePath: fields.inspirationFigurePath },
        { type: 'text', key: 'notes', label: 'Design Notes', body: fields.notesBody ?? '', figurePath: fields.notesFigurePath },
        { type: 'forza_recipe', key: 'recipe', label: 'Tune / Build Parts', tuneName: fields.tuneName ?? '', shareCode: fields.shareCode ?? '', coreSpecs: fields.coreSpecs ?? {}, upgrades: fields.upgrades ?? [], adjustments: fields.adjustments ?? [] },
      ],
    }
    const created = await api.createCard(newCard)
    cards.value.push(created)
    return created
  }

  async function deleteCard(id: string) {
    await api.deleteCard(id)
    const idx = cards.value.findIndex(c => c.id === id)
    if (idx !== -1) cards.value.splice(idx, 1)
    delete snapshots[id]
  }

  async function save(id: string) {
    const c = byId(id)
    if (!c) return

    // Paths present in the last save but absent now were deleted — collect before saving.
    const snap = snapshots[id]
    const orphans: string[] = []
    if (snap) {
      const currentPaths = new Set(
        c.images.flatMap(i => [i.path, i.thumbPath, i.stagePath].filter(Boolean) as string[])
      )
      for (const img of (JSON.parse(snap) as Card).images) {
        for (const p of [img.path, img.thumbPath, img.stagePath]) {
          if (p && !currentPaths.has(p)) orphans.push(p)
        }
      }
    }

    await api.saveCard(c)
    snapshots[id] = JSON.stringify(c)
    if (orphans.length) await api.deleteImages(orphans).catch(() => {})
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

  function restoreImageOrders(id: string, snapshot: { id: string; order: number }[]) {
    const c = byId(id)
    if (!c) return
    const orderMap = new Map(snapshot.map(s => [s.id, s.order]))
    for (const img of c.images) {
      const o = orderMap.get(img.id)
      if (o !== undefined) img.order = o
    }
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
    included = true,
  ) {
    const c = byId(cardId)
    if (!c) return
    const maxOrder = c.images.reduce((m, i) => Math.max(m, i.order), -1)
    c.images.push({ id: `${cardId}-${Date.now()}`, path, thumbPath, stagePath, order: maxOrder + 1, included })
  }

  function setColor(id: string, key: string, color: string | undefined) {
    const c = byId(id)
    if (!c) return
    if (!c.colors) c.colors = {}
    if (color) c.colors[key] = color
    else delete c.colors[key]
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

  // Distinct subtitle values at a given dot-segment position, excluding the
  // legend template card. Position 0 = Make/Model, 1 = Technique, etc.
  function allSubtitleSegments(position: number): string[] {
    const seen = new Set<string>()
    for (const c of cards.value) {
      if (c.isLegend) continue
      const parts = (c.subtitle ?? '').split(' · ')
      const v = parts[position]?.trim()
      if (v) seen.add(v)
    }
    return [...seen].sort()
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
    byId, load, save, deleteCard, createNewCard,
    takeSnapshot, restoreSnapshot, setFigure,
    toggleFavorite, setLeadImage, reorderImages, restoreImageOrders,
    removeImage, toggleImageIncluded, addImageToPool,
    setColor,
    addTag, removeTag, addCollection, removeCollection,
    allTagValues, allCollectionValues, allSectionKeys, allSubtitleSegments,
  }
})
