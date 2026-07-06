<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useFilterStore, COLOR_TAXONOMY, type LiveryColor } from '../stores/filters'
import { useCardsStore } from '../stores/cards'
import { useAuthStore } from '../stores/auth'
import { useModalStore } from '../stores/modal'
import { useTuneTypesStore } from '../stores/tune-types'
import { api } from '../api'

const filters = useFilterStore()
const store = useCardsStore()
const auth = useAuthStore()
const modal = useModalStore()
const tuneTypes = useTuneTypesStore()

async function fetchCount() {
  if (!auth.isAuthenticated) return
  try {
    const all = await api.adminListSuggestions()
    modal.pendingSuggestionCount = all.filter(s => s.status === 'pending').length
  } catch {}
}

onMounted(() => {
  fetchCount()
  tuneTypes.load()
})
watch(() => auth.isAuthenticated, fetchCount)

function toggleColor(c: LiveryColor) {
  filters.activeColor = filters.activeColor === c ? null : c
}
function toggleTuneType(name: string) {
  filters.activeTuneTypeName = filters.activeTuneTypeName === name ? null : name
}
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

  <p class="bug-flyout-label bug-flyout-label-2">Color</p>
  <div class="filter-color-grid">
    <button
      v-for="c in COLOR_TAXONOMY"
      :key="c"
      class="filter-color-chip"
      :class="{ 'filter-color-chip--active': filters.activeColor === c }"
      @click="toggleColor(c)"
    >{{ c }}</button>
  </div>

  <template v-if="tuneTypes.all.length">
    <p class="bug-flyout-label bug-flyout-label-2">Tune type</p>
    <div class="filter-tune-grid">
      <button
        v-for="t in tuneTypes.all"
        :key="t.id"
        class="filter-color-chip"
        :class="{ 'filter-color-chip--active': filters.activeTuneTypeName === t.name }"
        @click="toggleTuneType(t.name)"
      >{{ t.name }}</button>
    </div>
  </template>
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
.filter-color-grid,
.filter-tune-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 2px 0 4px;
}
.filter-color-chip {
  font: 10px/1 'JetBrains Mono', monospace;
  padding: 3px 7px;
  border-radius: 3px;
  border: 1px solid var(--panel-edge);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  transition: border-color .12s, color .12s, background .12s;
  white-space: nowrap;
}
.filter-color-chip:hover {
  border-color: var(--accent);
  color: var(--fg);
}
.filter-color-chip--active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--accent);
}

.sugg-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 14px;
  height: 14px;
  padding: 0 3px;
  background: #2d6a3f;
  color: #a8d8b0;
  border-radius: 3px;
  font-size: 9px;
  font-weight: bold;
  flex-shrink: 0;
  font-family: 'JetBrains Mono', monospace;
}
</style>
