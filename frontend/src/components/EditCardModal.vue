<script setup lang="ts">
import { ref, watch, computed, reactive } from 'vue'
import type { Card, ForzaRecipeSection } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import CollapsibleSection from './CollapsibleSection.vue'
import RecipeSection from './RecipeSection.vue'

const props = defineProps<{ card: Card }>()
const emit = defineEmits<{ close: [] }>()

const store = useCardsStore()
const ui = useUiStore()

// Buffered local copies — only applied on Save so Cancel is clean
const name       = ref('')
const subtitle   = ref('')
const collections = ref<string[]>([])
const collectionInput = ref('')
const tags        = ref<string[]>([])
const tagInput    = ref('')
const inspirationBody = ref('')
const notesBody   = ref('')

const saving      = ref(false)
const error       = ref('')
const confirmDelete = ref(false)

// Recipe is handled live by RecipeSection; snapshot here lets Cancel revert it
const recipeSection = computed(() =>
  props.card.sections.find(s => s.type === 'forza_recipe') as ForzaRecipeSection | undefined
)
const recipeSnapshot = ref('')
const sectionOpen = reactive({ insp: true, notes: true, recipe: true })

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
  const inspiration = c.sections.find(s => s.key === 'inspiration')
  const notes = c.sections.find(s => s.key === 'notes')
  const recipe = c.sections.find(s => s.type === 'forza_recipe')
  name.value        = c.name
  subtitle.value    = c.subtitle
  collections.value = [...c.collections]
  tags.value        = [...c.tags]
  inspirationBody.value = inspiration?.type === 'text' ? inspiration.body : ''
  notesBody.value   = notes?.type === 'text' ? notes.body : ''
  if (recipe) recipeSnapshot.value = JSON.stringify(recipe)
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
    // Recipe already mutated live by RecipeSection — no copy needed
    await store.save(c.id)
    ui.clearCardDirty(c.id)
    emit('close')
  } catch (e) {
    error.value = (e as Error).message
  } finally {
    saving.value = false
  }
}

function onCancel() {
  // Restore recipe from snapshot so any live RecipeSection changes are discarded
  const recipe = props.card.sections.find(s => s.type === 'forza_recipe')
  if (recipe?.type === 'forza_recipe' && recipeSnapshot.value) {
    const snap = JSON.parse(recipeSnapshot.value) as ForzaRecipeSection
    recipe.tuneName  = snap.tuneName
    recipe.shareCode = snap.shareCode
    recipe.showStock = snap.showStock
    for (const k of Object.keys(snap.coreSpecs)) recipe.coreSpecs[k] = snap.coreSpecs[k]
    for (const k of Object.keys(recipe.coreSpecs)) {
      if (!(k in snap.coreSpecs)) delete recipe.coreSpecs[k]
    }
    recipe.upgrades.splice(0, recipe.upgrades.length, ...snap.upgrades)
    recipe.adjustments.splice(0, recipe.adjustments.length, ...snap.adjustments)
  }
  emit('close')
}

async function onDelete() {
  await store.deleteCard(props.card.id)
  emit('close')
}
</script>

<template>
  <div class="nc-overlay open" @click.self="onCancel">
    <div class="card nc-modal-card">
      <button class="nc-close" @click="onCancel">×</button>

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

      <!-- Inspiration -->
      <CollapsibleSection label="Inspiration" section-key="modal-insp" v-model:open="sectionOpen.insp">
        <div class="section-body">
          <textarea class="nc-textarea" v-model="inspirationBody" rows="4"
            placeholder="The why/how/when behind the build, told with some license." />
        </div>
      </CollapsibleSection>

      <!-- Design Notes -->
      <CollapsibleSection label="Design Notes" section-key="modal-notes" v-model:open="sectionOpen.notes">
        <div class="section-body">
          <textarea class="nc-textarea" v-model="notesBody" rows="4"
            placeholder="Technique commentary — how it was built, material/layering choices." />
        </div>
      </CollapsibleSection>

      <!-- Tune / Build Parts — RecipeSection for full edit-mode parity -->
      <CollapsibleSection label="Tune / Build Parts" section-key="modal-recipe" v-model:open="sectionOpen.recipe">
        <RecipeSection v-if="recipeSection" :recipe="recipeSection" />
      </CollapsibleSection>

      <!-- Footer -->
      <div class="nc-footer">
        <p v-if="error" class="nc-error">{{ error }}</p>
        <div class="nc-actions ec-actions">
          <button class="ec-btn-delete" type="button" @click="confirmDelete = true">Delete Card</button>
          <div class="ec-right">
            <button class="nc-btn-cancel" type="button" @click="onCancel">Cancel</button>
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
