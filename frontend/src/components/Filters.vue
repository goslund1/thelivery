<script setup lang="ts">
import { useFilterStore } from '../stores/filters'
import { useCardsStore } from '../stores/cards'

const filters = useFilterStore()
const store = useCardsStore()
</script>

<template>
  <p class="bug-flyout-label">Show sections</p>
  <label v-for="s in store.allSectionKeys()" :key="s.key" class="bug-check">
    <input
      type="checkbox"
      :checked="filters.sectionExpanded[s.key]"
      @change="filters.setSectionExpanded(s.key, ($event.target as HTMLInputElement).checked)"
    />
    {{ s.label }}
  </label>
  <label class="bug-check">
    <input type="checkbox" v-model="filters.upgradesExpanded" /> Upgrades
  </label>

  <p class="bug-flyout-label bug-flyout-label-2">Collections</p>
  <label v-for="c in store.allCollectionValues()" :key="c" class="bug-check">
    <input
      type="checkbox"
      :checked="!filters.disabledCollections.has(c)"
      @change="filters.toggleCollection(c)"
    />
    {{ c }}
  </label>
  <label class="bug-check">
    <input type="checkbox" v-model="filters.favoritesOnly" /> ★ Favorites only
  </label>
</template>
