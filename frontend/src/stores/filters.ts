import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useCardsStore } from './cards'

export const useFilterStore = defineStore('filters', () => {
  const allExpanded = ref(false)
  const sectionExpanded = ref<Record<string, boolean>>({})
  const upgradesExpanded = ref(false)

  const favoritesOnly = ref(false)
  const disabledCollections = ref<Set<string>>(new Set())

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

  function isCardVisible(collections: string[], isFavorite: boolean) {
    if (favoritesOnly.value && !isFavorite) return false
    if (!collections.length) return true
    return collections.some(c => !disabledCollections.value.has(c))
  }

  return {
    allExpanded, sectionExpanded, upgradesExpanded,
    favoritesOnly, disabledCollections,
    toggleAll, setSectionExpanded, toggleCollection, isCardVisible,
  }
})
