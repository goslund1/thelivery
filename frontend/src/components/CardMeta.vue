<script setup lang="ts">
import { computed } from 'vue'
import type { Livery } from '../types'
import { useLiveriesStore } from '../stores/liveries'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'

const props = defineProps<{ livery: Livery }>()
const store = useLiveriesStore()
const ui = useUiStore()

const catalogNo = computed(() => String(props.livery.catalogNumber).padStart(3, '0'))

function toggleFav() {
  store.toggleFavorite(props.livery.id)
  ui.markCardDirty(props.livery.id)
}
function removeCollection(c: string) {
  store.removeCollection(props.livery.id, c)
  ui.markCardDirty(props.livery.id)
}
</script>

<template>
  <div class="card-meta">
    <div>
      <p class="card-number">
        CATALOG <span>NO. {{ catalogNo }}</span>
        <span
          v-for="c in livery.collections"
          :key="c"
          class="collection-badge chip"
          data-chip-type="collection"
        >{{ c }}<button class="chip-remove" type="button" @click="removeCollection(c)">×</button></span>
        <button class="chip-add" data-chip-type="collection" type="button" @click="ui.openChipPicker(livery.id, 'collection')">+</button>
      </p>
      <EditableText tag="h2" class="card-title" v-model="livery.name" />
      <EditableText tag="p" class="card-sub" v-model="livery.subtitle" />
    </div>
    <div class="card-meta-actions">
      <button class="fav-star" :class="{ favorited: livery.isFavorite }" aria-label="Favorite this livery" @click="toggleFav">★</button>
      <button
        v-if="ui.isEditing"
        class="card-save-btn"
        :class="{ 'has-changes': ui.isCardDirty(livery.id) }"
        :disabled="!ui.isCardDirty(livery.id) || ui.saving"
        @click="ui.saveCard(livery.id)"
      >{{ ui.isCardDirty(livery.id) ? 'Save ↓' : 'Saved' }}</button>
    </div>
  </div>
</template>

<style scoped>
.card-meta-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
}
.card-save-btn {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
  white-space: nowrap;
  background: var(--panel);
  color: var(--steel);
  border: 1px solid var(--panel-edge);
  transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
}
.card-save-btn.has-changes {
  background: var(--build-it-bg);
  color: var(--ink);
  border-color: var(--build-it-border);
}
.card-save-btn.has-changes:hover {
  background: var(--build-it-bg-hover);
}
.card-save-btn:disabled {
  cursor: default;
  opacity: 0.55;
}
</style>
