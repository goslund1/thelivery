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
  <template v-if="auth.isAuthenticated">
    <button class="bug-check sugg-row" @click="modal.openSuggestionViewer()">
      <span class="sugg-badge">{{ modal.pendingSuggestionCount }}</span>
      Suggestions
    </button>
    <p class="bug-flyout-label" style="margin-top: 8px;">Show sections</p>
  </template>
  <p v-else class="bug-flyout-label">Show sections</p>

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

<style scoped>
.sugg-row {
  background: none;
  border: none;
  width: 100%;
  text-align: left;
  transition: color .12s;
}
.sugg-row:hover { color: var(--accent); }
.sugg-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 14px;
  height: 14px;
  padding: 0 3px;
  background: var(--danger);
  color: #fff;
  border-radius: 3px;
  font-size: 9px;
  font-weight: bold;
  flex-shrink: 0;
  font-family: 'JetBrains Mono', monospace;
}
</style>
