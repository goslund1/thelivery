<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'
import EditCardModal from './EditCardModal.vue'
import SubtitleEditor from './SubtitleEditor.vue'
import { refreshTip } from '../composables/tooltip'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const catalogNo = computed(() => String(props.card.catalogNumber).padStart(3, '0'))
const editOpen = ref(false)

function formatShareCode(raw: string): string {
  const d = raw.replace(/\D/g, '').slice(0, 9)
  if (d.length <= 3) return d
  if (d.length <= 6) return `${d.slice(0, 3)} ${d.slice(3)}`
  return `${d.slice(0, 3)} ${d.slice(3, 6)} ${d.slice(6)}`
}
function onLiveryCodeInput(e: Event) {
  const input = e.target as HTMLInputElement
  const formatted = formatShareCode(input.value)
  store.setLiveryShareCode(props.card.id, formatted)
  input.value = formatted
  ui.markCardDirty(props.card.id)
}

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
      <div v-if="ui.isEditing || card.liveryShareCode" class="plate livery-code-plate">
        SHARE CODE:
        <input
          v-if="ui.isEditing"
          class="livery-code-input"
          :value="card.liveryShareCode"
          @input="onLiveryCodeInput"
          placeholder="000 000 000"
          maxlength="11"
          spellcheck="false"
        />
        <b v-else>{{ card.liveryShareCode || '—' }}</b>
      </div>
      <SubtitleEditor v-model="card.subtitle" />
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
.livery-code-plate {
  margin: 4px 0 6px;
}
.livery-code-input {
  background: none;
  border: none;
  border-bottom: 1px solid var(--panel-edge);
  color: var(--magenta);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: bold;
  letter-spacing: .08em;
  padding: 0 2px;
  width: 9em;
}
.livery-code-input:focus {
  outline: none;
  border-bottom-color: var(--gold);
}
.livery-code-input::placeholder { opacity: 0.35; font-weight: normal; }

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
