<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import EditableText from './EditableText.vue'
import EditCardModal from './EditCardModal.vue'
import SubtitleEditor from './SubtitleEditor.vue'
import { refreshTip } from '../composables/tooltip'
import { formatShareCode } from '../utils/shareCode'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()
const modal = useModalStore()

const catalogNo = computed(() => String(props.card.catalogNumber).padStart(3, '0'))
const editOpen = ref(false)

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

const ACCENT_PRESETS = [
  { label: 'Gold',    color: '#c9a227' },
  { label: 'Magenta', color: '#e83d9c' },
  { label: 'Blue',    color: '#5b8fb0' },
]

function setAccent(color: string | undefined) {
  store.setAccentOverride(props.card.id, color)
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
        <button class="chip-add" data-chip-type="collection" type="button" @click="modal.openChipPicker(card.id, 'collection')">+</button>
      </p>
      <EditableText v-if="ui.isEditing" tag="h2" class="card-title" v-model="card.name" />
      <h2 v-else class="card-title card-title--shareable" v-tip-up="'Click for sharing options'" @click="modal.openShare(card.id)">{{ card.name }}</h2>
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
      <div v-if="ui.isEditing" class="accent-picker">
        <button
          v-for="p in ACCENT_PRESETS"
          :key="p.color"
          class="accent-chip"
          :class="{ active: card.accentOverride === p.color }"
          :style="{ '--chip-color': p.color }"
          :title="p.label"
          @click="setAccent(card.accentOverride === p.color ? undefined : p.color)"
        ></button>
        <button v-if="card.accentOverride" class="accent-clear" title="Reset to theme accent" @click="setAccent(undefined)">×</button>
      </div>
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
  color: var(--highlight);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: bold;
  letter-spacing: .08em;
  padding: 0 2px;
  width: 9em;
}
.livery-code-input:focus {
  outline: none;
  border-bottom-color: var(--accent);
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
  color: var(--muted);
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
  border-color: var(--accent);
  color: var(--accent);
}

.accent-picker {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 6px 0 2px;
}
.accent-chip {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid transparent;
  background: var(--chip-color);
  cursor: pointer;
  padding: 0;
  transition: transform .12s, border-color .12s, box-shadow .12s;
}
.accent-chip:hover { transform: scale(1.2); }
.accent-chip.active { border-color: var(--fg); box-shadow: 0 0 0 1px var(--chip-color); }

.accent-clear {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  padding: 0 2px;
  transition: color .12s;
}
.accent-clear:hover { color: var(--fg); }

.card-title--shareable {
  cursor: pointer;
  transition: color 0.15s;
}
.card-title--shareable:hover { color: var(--accent); }
</style>
