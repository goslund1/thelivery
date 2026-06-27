<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'

const props = defineProps<{ card: Card }>()
const emit = defineEmits<{ close: [] }>()

const store = useCardsStore()
const ui = useUiStore()

// Local copies — changes only apply on Save
const name       = ref('')
const subtitle   = ref('')
const collections = ref<string[]>([])
const collectionInput = ref('')
const tags        = ref<string[]>([])
const tagInput    = ref('')
const inspirationBody = ref('')
const notesBody   = ref('')
const tuneName    = ref('')
const shareCode   = ref('')
const CORE_SPEC_KEYS = ['Drivetrain', 'Engine', 'Transmission', 'Tires', 'Suspension']
const coreSpecs   = ref<Record<string, string>>({})

const saving      = ref(false)
const error       = ref('')
const confirmDelete = ref(false)

const existingCollections = computed(() => store.allCollectionValues())
const existingTags        = computed(() => store.allTagValues())

const filteredTags = computed(() => {
  const q = tagInput.value.toLowerCase().trim()
  const available = existingTags.value.filter(t => !tags.value.includes(t))
  return q ? available.filter(t => t.toLowerCase().includes(q)) : available
})
const canCreateTag = computed(() => {
  const q = tagInput.value.trim()
  return q && !tags.value.includes(q) && !existingTags.value.some(t => t.toLowerCase() === q.toLowerCase())
})

function populate() {
  const c = props.card
  const recipe = c.sections.find(s => s.type === 'forza_recipe')
  const inspiration = c.sections.find(s => s.key === 'inspiration')
  const notes = c.sections.find(s => s.key === 'notes')
  name.value        = c.name
  subtitle.value    = c.subtitle
  collections.value = [...c.collections]
  tags.value        = [...c.tags]
  inspirationBody.value = inspiration?.type === 'text' ? inspiration.body : ''
  notesBody.value   = notes?.type === 'text' ? notes.body : ''
  tuneName.value    = recipe?.type === 'forza_recipe' ? recipe.tuneName : ''
  shareCode.value   = recipe?.type === 'forza_recipe' ? recipe.shareCode : ''
  coreSpecs.value   = recipe?.type === 'forza_recipe'
    ? Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, recipe.coreSpecs[k] ?? '']))
    : Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, '']))
  confirmDelete.value = false
  error.value = ''
}

watch(() => props.card, populate, { immediate: true })

// Collections
function toggleCollection(c: string) {
  collections.value = collections.value.includes(c)
    ? collections.value.filter(x => x !== c)
    : [...collections.value, c]
}
function addCollectionFromInput() {
  const v = collectionInput.value.trim()
  if (v && !collections.value.includes(v)) collections.value.push(v)
  collectionInput.value = ''
}
function onCollectionKey(e: KeyboardEvent) {
  if (e.key === 'Enter') { e.preventDefault(); addCollectionFromInput() }
  if (e.key === 'Backspace' && !collectionInput.value) collections.value.pop()
}

// Tags
function addTag(tag: string) {
  const t = tag.trim()
  if (t && !tags.value.includes(t)) tags.value.push(t)
  tagInput.value = ''
}
function removeTag(tag: string) { tags.value = tags.value.filter(t => t !== tag) }
function onTagKey(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault()
    const q = tagInput.value.trim()
    if (!q) return
    filteredTags.value.length > 0 ? addTag(filteredTags.value[0]) : canCreateTag.value && addTag(q)
  }
  if (e.key === 'Backspace' && !tagInput.value) tags.value.pop()
}

async function onSave() {
  if (!name.value.trim()) { error.value = 'Name is required.'; return }
  saving.value = true
  error.value = ''
  try {
    const c = props.card
    c.name = name.value.trim()
    c.subtitle = subtitle.value.trim()
    c.collections = collections.value
    c.tags = tags.value
    const insp = c.sections.find(s => s.key === 'inspiration')
    if (insp?.type === 'text') insp.body = inspirationBody.value
    const notes = c.sections.find(s => s.key === 'notes')
    if (notes?.type === 'text') notes.body = notesBody.value
    const recipe = c.sections.find(s => s.type === 'forza_recipe')
    if (recipe?.type === 'forza_recipe') {
      recipe.tuneName = tuneName.value.trim()
      recipe.shareCode = shareCode.value.trim()
      for (const k of CORE_SPEC_KEYS) recipe.coreSpecs[k] = coreSpecs.value[k] ?? ''
    }
    await store.save(c.id)
    ui.clearCardDirty(c.id)
    emit('close')
  } catch (e) {
    error.value = (e as Error).message
  } finally {
    saving.value = false
  }
}

async function onDelete() {
  await store.deleteCard(props.card.id)
  emit('close')
}
</script>

<template>
  <div class="nc-overlay open" @click.self="emit('close')">
    <div class="card nc-modal-card">
      <button class="nc-close" @click="emit('close')">×</button>

      <!-- Card meta -->
      <div class="card-meta nc-card-meta">
        <div class="nc-meta-inner">
          <p class="card-number">
            CATALOG <span>NO. {{ String(card.catalogNumber).padStart(3, '0') }}</span>
            <span v-for="c in collections" :key="c" class="collection-badge nc-badge">
              {{ c }}<button class="nc-x" type="button" @click="toggleCollection(c)">×</button>
            </span>
            <input class="nc-col-inline" v-model="collectionInput" placeholder="+ collection"
              @keydown="onCollectionKey" @blur="addCollectionFromInput" />
          </p>
          <div v-if="existingCollections.length" class="nc-col-picks">
            <button v-for="c in existingCollections" :key="c"
              class="nc-col-pick" :class="{ 'nc-col-pick--on': collections.includes(c) }"
              type="button" @click="toggleCollection(c)">{{ c }}</button>
          </div>
          <input class="card-title nc-title-input" v-model="name" placeholder="Livery Name" @keydown.enter.prevent />
          <input class="card-sub nc-sub-input" v-model="subtitle" placeholder="Make/Model · Base Coat or Technique · Style · Rims" />
        </div>
      </div>

      <!-- Tags -->
      <div class="tag-cloud nc-tag-cloud">
        <span v-for="t in tags" :key="t" class="tag nc-tag-sel">
          {{ t }}<button class="nc-x" type="button" @click="removeTag(t)">×</button>
        </span>
        <input class="nc-tag-input" v-model="tagInput" placeholder="Filter or add tags…" @keydown="onTagKey" />
        <button v-for="t in filteredTags" :key="t" class="nc-tag-opt" type="button" @click="addTag(t)">{{ t }}</button>
        <button v-if="canCreateTag" class="nc-tag-opt nc-tag-new" type="button" @click="addTag(tagInput)">+ "{{ tagInput.trim() }}"</button>
      </div>

      <!-- Sections -->
      <div class="nc-section">
        <div class="nc-section-head">Inspiration</div>
        <div class="section-body">
          <textarea class="nc-textarea" v-model="inspirationBody" rows="4"
            placeholder="The why/how/when behind the build, told with some license." />
        </div>
      </div>

      <div class="nc-section">
        <div class="nc-section-head">Design Notes</div>
        <div class="section-body">
          <textarea class="nc-textarea" v-model="notesBody" rows="4"
            placeholder="Technique commentary — how it was built, material/layering choices." />
        </div>
      </div>

      <div class="nc-section">
        <div class="nc-section-head">Tune / Build Parts</div>
        <div class="section-body">
          <div class="nc-recipe-row">
            <div class="nc-recipe-field">
              <label class="nc-recipe-label">Tune Name</label>
              <input class="nc-recipe-input" v-model="tuneName" placeholder="Saved name in-game" />
            </div>
            <div class="nc-recipe-field nc-recipe-field--narrow">
              <label class="nc-recipe-label">Share Code</label>
              <input class="nc-recipe-input" v-model="shareCode" placeholder="000 000 000" />
            </div>
          </div>
          <div class="nc-specs-grid">
            <div v-for="key in CORE_SPEC_KEYS" :key="key" class="nc-spec">
              <label class="nc-recipe-label">{{ key }}</label>
              <input class="nc-recipe-input" v-model="coreSpecs[key]"
                :placeholder="`e.g. ${key === 'Drivetrain' ? 'AWD' : key === 'Tires' ? 'Rally' : 'stock'}`" />
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="nc-footer">
        <p v-if="error" class="nc-error">{{ error }}</p>
        <div class="nc-actions ec-actions">
          <button class="ec-btn-delete" type="button" @click="confirmDelete = true">Delete Card</button>
          <div class="ec-right">
            <button class="nc-btn-cancel" type="button" @click="emit('close')">Cancel</button>
            <button class="nc-btn-create" type="button" :disabled="saving" @click="onSave">
              {{ saving ? 'Saving…' : 'Save Changes →' }}
            </button>
          </div>
        </div>
      </div>

    </div>
  </div>

  <Teleport to="body">
    <div v-if="confirmDelete" class="ec-delete-overlay" @click.self="confirmDelete = false">
      <div class="ec-delete-dialog">
        <p class="ec-delete-msg">Delete <strong>{{ card.name }}</strong>?</p>
        <p class="ec-delete-sub">This cannot be undone.</p>
        <div class="ec-delete-actions">
          <button class="nc-btn-cancel" type="button" @click="confirmDelete = false">Cancel</button>
          <button class="ec-btn-delete ec-btn-delete--confirm" type="button" @click="onDelete">Yes, delete it</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ec-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ec-right {
  display: flex;
  gap: 10px;
}
.ec-btn-delete {
  background: transparent;
  border: 1px solid #c0392b;
  border-radius: 4px;
  color: #c0392b;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: .08em;
  text-transform: uppercase;
  padding: 10px 18px;
  cursor: pointer;
  transition: background .15s, color .15s;
}
.ec-btn-delete:hover, .ec-btn-delete--confirm { background: #c0392b; color: #fff; }

/* Centered delete confirm popup */
.ec-delete-overlay {
  position: fixed;
  inset: 0;
  z-index: 1200;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
}
.ec-delete-dialog {
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 6px;
  padding: 28px 32px 24px;
  max-width: 360px;
  width: 90vw;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.7);
}
.ec-delete-msg {
  font-family: 'JetBrains Mono', monospace;
  font-size: 14px;
  color: var(--paper);
  margin: 0 0 6px;
}
.ec-delete-sub {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--steel);
  margin: 0 0 22px;
}
.ec-delete-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
