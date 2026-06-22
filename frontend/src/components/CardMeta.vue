<script setup lang="ts">
import { computed } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'
import { refreshTip } from '../composables/tooltip'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const catalogNo = computed(() => String(props.card.catalogNumber).padStart(3, '0'))

function toggleFav() {
  store.toggleFavorite(props.card.id)
  ui.markCardDirty(props.card.id)
  // Star stays in place → sync its tooltip text to the new state (Rule B).
  refreshTip(props.card.isFavorite ? 'Unmark as favorite' : 'Mark as favorite')
}
function removeCollection(c: string) {
  store.removeCollection(props.card.id, c)
  ui.markCardDirty(props.card.id)
}
</script>

<template>
  <div class="card-meta">
    <div>
      <p class="card-number">
        CATALOG <span>NO. {{ catalogNo }}</span>
        <span
          v-for="c in card.collections"
          :key="c"
          class="collection-badge chip"
          data-chip-type="collection"
        >{{ c }}<button class="chip-remove" type="button" @click="removeCollection(c)">×</button></span>
        <button class="chip-add" data-chip-type="collection" type="button" @click="ui.openChipPicker(card.id, 'collection')">+</button>
      </p>
      <EditableText tag="h2" class="card-title" v-model="card.name" />
      <EditableText tag="p" class="card-sub" v-model="card.subtitle" />
    </div>
    <div class="card-meta-actions">
      <button class="fav-star" :class="{ favorited: card.isFavorite }" aria-label="Favorite this card" v-tip="() => card.isFavorite ? 'Unmark as favorite' : 'Mark as favorite'" @click="toggleFav">★</button>
      <button
        v-if="ui.isEditing"
        class="card-save-btn"
        :class="{ 'has-changes': ui.isCardDirty(card.id) }"
        :disabled="!ui.isCardDirty(card.id) || ui.saving"
        @click="ui.saveCard(card.id)"
      >{{ ui.isCardDirty(card.id) ? 'Save ↓' : 'Saved' }}</button>
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
