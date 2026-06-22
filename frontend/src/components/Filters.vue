<script setup lang="ts">
import { useUiStore } from '../stores/ui'
import { useLiveriesStore } from '../stores/liveries'

const ui = useUiStore()
const store = useLiveriesStore()

const sections = [
  { key: 'inspiration', label: 'Inspiration' },
  { key: 'notes', label: 'Design Notes' },
  { key: 'recipe', label: 'Recipe' },
] as const
</script>

<template>
  <p class="bug-flyout-label">Show sections</p>
  <label v-for="s in sections" :key="s.key" class="bug-check">
    <input
      type="checkbox"
      :checked="ui.sectionExpanded[s.key]"
      @change="ui.setSectionExpanded(s.key, ($event.target as HTMLInputElement).checked)"
    />
    {{ s.label }}
  </label>
  <label class="bug-check">
    <input type="checkbox" v-model="ui.upgradesExpanded" /> Upgrades
  </label>

  <p class="bug-flyout-label bug-flyout-label-2">Collections</p>
  <label v-for="c in store.allCollectionValues()" :key="c" class="bug-check">
    <input
      type="checkbox"
      :checked="!ui.disabledCollections.has(c)"
      @change="ui.toggleCollection(c)"
    />
    {{ c }}
  </label>
  <label class="bug-check">
    <input type="checkbox" v-model="ui.favoritesOnly" /> ★ Favorites only
  </label>
</template>
