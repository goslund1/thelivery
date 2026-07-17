import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { useCardsStore } from './cards'
import { useLiveriesStore } from './liveries'
import type { Card } from '../types'

export const COLOR_TAXONOMY = [
  'Red', 'Blue', 'Green', 'Yellow', 'Orange', 'Purple', 'Pink',
  'White', 'Black', 'Silver', 'Grey', 'Gold', 'Bronze', 'Teal', 'Multi',
] as const
export type LiveryColor = typeof COLOR_TAXONOMY[number]

export const useFilterStore = defineStore('filters', () => {
  const allExpanded = ref(false)
  const sectionExpanded = ref<Record<string, boolean>>({})
  const upgradesExpanded = ref(false)

  const favoritesOnly = ref(false)
  const disabledCollections = ref<Set<string>>(new Set())

  // Color + tune-type axes (null = no filter active)
  const activeColor = ref<LiveryColor | null>(null)
  const activeTuneTypeName = ref<string | null>(null)

  function toggleAll() {
    allExpanded.value = !allExpanded.value
    const v = allExpanded.value
    for (const { key } of useCardsStore().allSectionKeys()) sectionExpanded.value[key] = v
    upgradesExpanded.value = v
  }

  function setSectionExpanded(key: string, v: boolean) {
    sectionExpanded.value[key] = v
  }

  function toggleCollection(name: string) {
    const s = new Set(disabledCollections.value)
    s.has(name) ? s.delete(name) : s.add(name)
    disabledCollections.value = s
  }

  function isCardVisible(card: Card) {
    if (favoritesOnly.value && !card.isFavorite) return false

    if (card.collections.length && !card.collections.some(c => !disabledCollections.value.has(c))) return false

    if (activeColor.value) {
      const liveriesStore = useLiveriesStore()
      const matches = card.images.some(img => {
        if (!img.liveryId) return false
        const livery = liveriesStore.get(img.liveryId)
        return livery?.colorPrimary === activeColor.value || livery?.colorSecondary === activeColor.value
      })
      if (!matches) return false
    }

    if (activeTuneTypeName.value) {
      const sections = card.sections.filter(s => s.type === 'forza_recipe')
      const matches = sections.some(s => {
        if (s.type !== 'forza_recipe') return false
        return s.variants?.some(v => v.tuneType === activeTuneTypeName.value)
      })
      if (!matches) return false
    }

    return true
  }

  const hasActiveFilters = computed(() =>
    favoritesOnly.value
    || disabledCollections.value.size > 0
    || activeColor.value !== null
    || activeTuneTypeName.value !== null
  )

  function resetFilters() {
    favoritesOnly.value = false
    disabledCollections.value = new Set()
    activeColor.value = null
    activeTuneTypeName.value = null
  }

  return {
    allExpanded, sectionExpanded, upgradesExpanded,
    favoritesOnly, disabledCollections,
    activeColor, activeTuneTypeName, hasActiveFilters,
    toggleAll, setSectionExpanded, toggleCollection, isCardVisible, resetFilters,
  }
})
