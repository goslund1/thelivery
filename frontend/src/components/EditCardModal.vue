<script setup lang="ts">
import { ref, watch, computed, reactive, onMounted, onUnmounted } from 'vue'
import type { Card, TextSection as TextSectionData, ForzaRecipeSection } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import CollapsibleSection from './CollapsibleSection.vue'
import TextSection from './TextSection.vue'
import RecipeSection from './RecipeSection.vue'
import SubtitleEditor from './SubtitleEditor.vue'

const props = defineProps<{ card: Card }>()
const emit = defineEmits<{ close: [] }>()

const store = useCardsStore()
const ui = useUiStore()
const modal = useModalStore()

// Buffered local copies for fields that need clean Cancel support
const name       = ref('')
const subtitle   = ref('')
const liveryShareCode = ref('')
const carId      = ref<string | null>(null)
const carIdSnapshot = ref<string | null>(null)
const collections = ref<string[]>([])
const collectionInput = ref('')
const tags        = ref<string[]>([])
const tagInput    = ref('')

const saving      = ref(false)
const error       = ref('')
const confirmDelete = ref(false)

// Text sections and recipe are handled live by their components;
// snapshots let Cancel revert any changes made inside the modal.
const inspirationSection = computed<TextSectionData | undefined>(() => {
  const s = props.card.sections.find(s => s.key === 'inspiration')
  return s?.type === 'text' ? s : undefined
})
const notesSection = computed<TextSectionData | undefined>(() => {
  const s = props.card.sections.find(s => s.key === 'notes')
  return s?.type === 'text' ? s : undefined
})
const recipeSection = computed<ForzaRecipeSection | undefined>(() => {
  const s = props.card.sections.find(s => s.type === 'forza_recipe')
  return s?.type === 'forza_recipe' ? s : undefined
})
// Non-null accessors for template bindings — v-if guards above guarantee non-null
const inspSect   = computed(() => inspirationSection.value!)
const notesSect  = computed(() => notesSection.value!)
const recipeSect = computed(() => recipeSection.value!)

const recipeResetToken = ref(0)

const inspSnapshot   = ref('')
const notesSnapshot  = ref('')
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

function formatShareCode(raw: string): string {
  const d = raw.replace(/\D/g, '').slice(0, 9)
  if (d.length <= 3) return d
  if (d.length <= 6) return `${d.slice(0, 3)} ${d.slice(3)}`
  return `${d.slice(0, 3)} ${d.slice(3, 6)} ${d.slice(6)}`
}
function onLiveryCodeInput(e: Event) {
  const input = e.target as HTMLInputElement
  const formatted = formatShareCode(input.value)
  liveryShareCode.value = formatted
  input.value = formatted
}

function populate() {
  const c = props.card
  name.value            = c.name
  subtitle.value        = c.subtitle
  liveryShareCode.value = c.liveryShareCode ?? ''
  carId.value = c.carId ?? null
  carIdSnapshot.value = c.carId ?? null
  collections.value = [...c.collections]
  tags.value        = [...c.tags]
  const insp   = c.sections.find(s => s.key === 'inspiration')
  const notes  = c.sections.find(s => s.key === 'notes')
  const recipe = c.sections.find(s => s.type === 'forza_recipe')
  if (insp)   inspSnapshot.value   = JSON.stringify(insp)
  if (notes)  notesSnapshot.value  = JSON.stringify(notes)
  if (recipe) recipeSnapshot.value = JSON.stringify(recipe)
  confirmDelete.value = false
  error.value = ''
}

watch(() => props.card, populate, { immediate: true })

onMounted(() => { document.body.style.overflow = 'hidden' })
onUnmounted(() => { document.body.style.overflow = '' })

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
    c.liveryShareCode = liveryShareCode.value.trim() || undefined
    c.carId = carId.value ?? undefined
    c.collections = collections.value
    c.tags = tags.value
    // Text sections and recipe already mutated live by their components
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
  // Restore text sections and recipe from snapshots
  const c = props.card
  const insp = c.sections.find(s => s.key === 'inspiration')
  if (insp?.type === 'text' && inspSnapshot.value) {
    const snap = JSON.parse(inspSnapshot.value)
    insp.body = snap.body
    insp.figurePath = snap.figurePath
  }
  const notes = c.sections.find(s => s.key === 'notes')
  if (notes?.type === 'text' && notesSnapshot.value) {
    const snap = JSON.parse(notesSnapshot.value)
    notes.body = snap.body
    notes.figurePath = snap.figurePath
  }
  const recipe = c.sections.find(s => s.type === 'forza_recipe')
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
    recipeResetToken.value++
  }
  c.carId = carIdSnapshot.value ?? undefined
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
      <button class="ec-btn-history" type="button" @click="modal.openHistory(card.id)">History</button>
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
          <div class="plate nc-livery-code-plate">
            SHARE CODE:
            <input
              class="nc-livery-code-input"
              :value="liveryShareCode"
              @input="onLiveryCodeInput"
              placeholder="000 000 000"
              maxlength="11"
              spellcheck="false"
            />
          </div>
          <SubtitleEditor v-model="subtitle" />
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

      <!-- Inspiration — full TextSection with gutter figure -->
      <CollapsibleSection label="Inspiration" section-key="modal-insp" v-model:open="sectionOpen.insp">
        <TextSection v-if="inspirationSection" :card-id="card.id" :section="inspSect" />
      </CollapsibleSection>

      <!-- Design Notes — full TextSection with gutter figure -->
      <CollapsibleSection label="Design Notes" section-key="modal-notes" v-model:open="sectionOpen.notes">
        <TextSection v-if="notesSection" :card-id="card.id" :section="notesSect" />
      </CollapsibleSection>

      <!-- Tune / Build Parts — full RecipeSection -->
      <CollapsibleSection label="Tune / Build Parts" section-key="modal-recipe" v-model:open="sectionOpen.recipe">
        <RecipeSection
          v-if="recipeSection"
          :recipe="recipeSect"
          :car-id="carId"
          :reset-token="recipeResetToken"
          @update:recipe="updated => Object.assign(recipeSect, updated)"
          @update:car-id="id => { carId = id }"
        />
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
    <div v-if="confirmDelete" class="conf-overlay open" @click.self="confirmDelete = false">
      <div class="conf-panel">
        <div class="conf-head">
          <span>Delete Card</span>
          <button class="conf-close" @click="confirmDelete = false">×</button>
        </div>
        <p class="conf-body"><strong style="color:var(--fg)">{{ card.name }}</strong> will be permanently deleted. This cannot be undone.</p>
        <div class="conf-actions">
          <button class="conf-btn conf-btn--discard" type="button" @click="onDelete">Yes, Delete It</button>
          <button class="conf-btn conf-btn--neutral" type="button" @click="confirmDelete = false">Cancel</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.nc-livery-code-plate {
  margin: 4px 0 6px;
}
.nc-livery-code-input {
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
.nc-livery-code-input:focus {
  outline: none;
  border-bottom-color: var(--accent);
}
.nc-livery-code-input::placeholder { opacity: 0.35; font-weight: normal; }

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
  border: 1px solid var(--danger);
  border-radius: 4px;
  color: var(--danger-bright);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: .08em;
  text-transform: uppercase;
  padding: 10px 18px;
  cursor: pointer;
  transition: background .15s, color .15s, border-color .15s;
}
.ec-btn-delete:hover { background: color-mix(in srgb, var(--danger) 20%, transparent); border-color: var(--danger-bright); }
.ec-btn-history {
  position: absolute;
  top: 18px;
  right: 60px;
  z-index: 10;
  background: transparent;
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: .08em;
  text-transform: uppercase;
  padding: 4px 10px;
  cursor: pointer;
  transition: border-color .15s, color .15s;
}
.ec-btn-history:hover { border-color: var(--accent); color: var(--accent); }

</style>
