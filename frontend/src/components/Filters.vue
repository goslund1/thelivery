<script setup lang="ts">
import { onMounted } from 'vue'
import { useFilterStore } from '../stores/filters'
import { useCardsStore } from '../stores/cards'
import { useAuthStore } from '../stores/auth'
import { useModalStore } from '../stores/modal'
import { api } from '../api'

const filters = useFilterStore()
const store = useCardsStore()
const auth = useAuthStore()
const modal = useModalStore()

onMounted(async () => {
  if (!auth.isAuthenticated) return
  try {
    const all = await api.adminListSuggestions()
    modal.pendingSuggestionCount = all.filter(s => s.status === 'pending').length
  } catch {}
})
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

  <template v-if="auth.isAuthenticated">
    <p class="bug-flyout-label bug-flyout-label-2">Admin</p>
    <button class="bug-check bug-check--action" @click="modal.openSuggestionViewer()">
      Tune Suggestions
      <span v-if="modal.pendingSuggestionCount" class="bug-sugg-badge">{{ modal.pendingSuggestionCount }}</span>
    </button>
  </template>
</template>

<style scoped>
.bug-check--action {
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--fg);
  font-family: inherit;
  font-size: inherit;
  padding: 2px 0;
  cursor: pointer;
  width: 100%;
  text-align: left;
}
.bug-check--action:hover { color: var(--accent); }
.bug-sugg-badge {
  background: var(--accent);
  color: var(--bg);
  border-radius: 8px;
  font-size: 10px;
  font-family: 'JetBrains Mono', monospace;
  font-weight: bold;
  padding: 0 5px;
  line-height: 16px;
}
</style>
