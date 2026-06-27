<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'
import EditCardModal from './EditCardModal.vue'
import { refreshTip } from '../composables/tooltip'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const catalogNo = computed(() => String(props.card.catalogNumber).padStart(3, '0'))
const editOpen = ref(false)

function toggleFav() {
  store.toggleFavorite(props.card.id)
  ui.markCardDirty(props.card.id)
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
      <button v-if="ui.isEditing" class="edit-card-btn" @click="editOpen = true">Edit Card</button>
      <button class="fav-star" :class="{ favorited: card.isFavorite }" aria-label="Favorite this card" v-tip="() => card.isFavorite ? 'Unmark as favorite' : 'Mark as favorite'" @click="toggleFav">★</button>
    </div>
  </div>

  <Teleport to="body">
    <EditCardModal v-if="editOpen" :card="card" @close="editOpen = false" />
  </Teleport>
</template>

<style scoped>
.card-meta-actions {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  gap: 10px;
  padding-top: 12px;
}

/* The fav-star is position:absolute in global CSS so it sits outside flow.
   This button needs to be absolute too, shifted left of the star. */
.edit-card-btn {
  position: absolute;
  top: 26px;
  right: 69px;
  background: transparent;
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .06em;
  padding: 4px 8px;
  cursor: pointer;
  z-index: 11;
  transition: border-color .15s, color .15s;
}
.edit-card-btn:hover {
  border-color: var(--gold);
  color: var(--gold);
}
</style>
