<script setup lang="ts">
import { useUiStore } from '../stores/ui'
import { useCardsStore } from '../stores/cards'

const ui = useUiStore()
const store = useCardsStore()
</script>

<template>
  <p class="bug-flyout-label">Show sections</p>
  <label v-for="s in store.allSectionKeys()" :key="s.key" class="bug-check">
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
