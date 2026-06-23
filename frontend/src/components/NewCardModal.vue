<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'

const store = useCardsStore()
const ui = useUiStore()

const name = ref('')
const subtitle = ref('')
const collections = ref<string[]>([])
const collectionInput = ref('')
const saving = ref(false)
const error = ref('')
const nameRef = ref<HTMLInputElement | null>(null)

watch(() => ui.newCardOpen, async (open) => {
  if (!open) return
  name.value = ''
  subtitle.value = ''
  collections.value = []
  collectionInput.value = ''
  error.value = ''
  await nextTick()
  nameRef.value?.focus()
})

function addCollection() {
  const v = collectionInput.value.trim()
  if (v && !collections.value.includes(v)) collections.value.push(v)
  collectionInput.value = ''
}

function removeCollection(c: string) {
  collections.value = collections.value.filter((x) => x !== c)
}

function onCollectionKey(e: KeyboardEvent) {
  if (e.key === 'Enter') { e.preventDefault(); addCollection() }
  if (e.key === 'Backspace' && !collectionInput.value) collections.value.pop()
}

async function onCreate() {
  if (!name.value.trim()) { error.value = 'Name is required.'; return }
  saving.value = true
  error.value = ''
  try {
    await store.createNewCard({
      name: name.value.trim(),
      subtitle: subtitle.value.trim(),
      collections: collections.value,
    })
    ui.closeNewCard()
    // Scroll to the new card at the bottom
    await nextTick()
    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' })
  } catch (e) {
    error.value = (e as Error).message
  } finally {
    saving.value = false
  }
}

function onCancel() { ui.closeNewCard() }
function onOverlay(e: MouseEvent) { if (e.target === e.currentTarget) onCancel() }
</script>

<template>
  <div class="image-picker" :class="{ open: ui.newCardOpen }" @click="onOverlay">
    <div class="image-picker-panel new-card-panel">

      <div class="image-picker-head">
        <span>New Card</span>
        <button class="image-picker-close" aria-label="Cancel" @click="onCancel">×</button>
      </div>

      <div class="nc-field">
        <label class="nc-label">Name</label>
        <input
          ref="nameRef"
          class="nc-input"
          v-model="name"
          placeholder="Livery name…"
          @keydown.enter="onCreate"
        />
      </div>

      <div class="nc-field">
        <label class="nc-label">Subtitle</label>
        <input
          class="nc-input"
          v-model="subtitle"
          placeholder="Make / model, technique…"
          @keydown.enter="onCreate"
        />
      </div>

      <div class="nc-field">
        <label class="nc-label">Collections</label>
        <div class="nc-chips">
          <span v-for="c in collections" :key="c" class="chip collection-badge">
            {{ c }}<button class="chip-remove" type="button" @click="removeCollection(c)">×</button>
          </span>
          <input
            class="nc-chip-input"
            v-model="collectionInput"
            placeholder="Type and press Enter…"
            @keydown="onCollectionKey"
            @blur="addCollection"
          />
        </div>
      </div>

      <p v-if="error" class="nc-error">{{ error }}</p>

      <div class="nc-actions">
        <button class="nc-cancel" @click="onCancel">Cancel</button>
        <button class="nc-create" :disabled="saving" @click="onCreate">
          {{ saving ? 'Creating…' : 'Create Card →' }}
        </button>
      </div>

    </div>
  </div>
</template>

<style scoped>
.new-card-panel {
  max-width: 520px;
}

.nc-field {
  margin-bottom: 18px;
}

.nc-label {
  display: block;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--steel);
  margin-bottom: 6px;
}

.nc-input {
  width: 100%;
  box-sizing: border-box;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--paper);
  font-family: inherit;
  font-size: 15px;
  padding: 10px 12px;
  outline: none;
  transition: border-color 0.15s ease;
}

.nc-input:focus {
  border-color: var(--gold);
}

.nc-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  padding: 8px 10px;
  min-height: 42px;
}

.nc-chip-input {
  background: transparent;
  border: none;
  outline: none;
  color: var(--paper);
  font-family: inherit;
  font-size: 13px;
  flex: 1;
  min-width: 120px;
}

.nc-error {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--danger);
  margin: -8px 0 14px;
}

.nc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
  padding-top: 18px;
  border-top: 1px solid var(--panel-edge);
}

.nc-cancel {
  background: transparent;
  border: 1px solid var(--panel-edge);
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
  transition: color 0.15s ease, border-color 0.15s ease;
}

.nc-cancel:hover {
  color: var(--paper);
  border-color: var(--steel);
}

.nc-create {
  background: var(--gold);
  border: none;
  color: var(--ink);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 10px 24px;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.nc-create:hover:not(:disabled) {
  opacity: 0.85;
}

.nc-create:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
