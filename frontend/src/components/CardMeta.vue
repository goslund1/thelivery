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
    <span v-if="card.isLegend" class="legend-flag">Field Legend — Not a real entry</span>
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
</style>
