<script setup lang="ts">
import { ref, reactive, computed, nextTick, watch, onUnmounted } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { errMsg } from '../utils/errMsg'
import type { ForzaRecipeSection } from '../types'
import CollapsibleSection from './CollapsibleSection.vue'
import RecipeSection from './RecipeSection.vue'
import SubtitleEditor from './SubtitleEditor.vue'

const store = useCardsStore()
const modal = useModalStore()

// Fields
const name = ref('')
const subtitle = ref('')
const selectedCollections = ref<string[]>([])
const collectionInput = ref('')
const selectedTags = ref<string[]>([])
const tagInput = ref('')
const inspirationBody = ref('')
const notesBody = ref('')

// Section figure images — paths stored here after user picks
const inspirationFigurePath = ref<string | null>(null)
const notesFigurePath = ref<string | null>(null)

// Creates the card in the DB lazily (on first photo action). Receives the session carId
// from the Photo Manager so the card can be tagged immediately.
async function ensureCard(carId: string | null): Promise<string> {
  if (importedCard.value) return importedCard.value.id
  saving.value = true
  try {
    const card = await store.createNewCard({
      name: name.value.trim() || 'New Card',
      subtitle: subtitle.value.trim(),
      collections: selectedCollections.value,
      tags: selectedTags.value,
      inspirationBody: inspirationBody.value.trim(),
      notesBody: notesBody.value.trim(),
      tuneName: recipe.value.tuneName.trim(),
      shareCode: recipe.value.shareCode.trim(),
      coreSpecs: { ...recipe.value.coreSpecs },
      upgrades: JSON.parse(JSON.stringify(recipe.value.upgrades)),
      adjustments: JSON.parse(JSON.stringify(recipe.value.adjustments)),
      carId: carId ?? undefined,
    })
    await store.save(card.id)
    importedCard.value = { id: card.id, name: card.name, subtitle: card.subtitle, collections: card.collections }
    return card.id
  } finally {
    saving.value = false
  }
}

function openPhotoManager() {
  modal.openManagePhotos(importedCard.value?.id ?? null, ensureCard)
}

function openPickerForSection(section: 'insp' | 'notes') {
  modal.openFigurePicker(
    importedCard.value?.id ?? null,
    null,
    (path) => {
      if (section === 'insp') inspirationFigurePath.value = path
      else notesFigurePath.value = path
    },
    ensureCard,
  )
}

// Recipe — ref so reassignment on modal-open triggers RecipeSection's props watcher for a clean reset
const CORE_SPEC_KEYS = ['Drivetrain', 'Engine', 'Transmission', 'Tires', 'Suspension']
function blankRecipe(): ForzaRecipeSection {
  return {
    type: 'forza_recipe',
    key: 'recipe',
    label: 'Tune / Build Parts',
    tuneName: '',
    shareCode: '',
    coreSpecs: Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, ''])),
    upgrades: [],
    adjustments: [],
  }
}
const recipe = ref<ForzaRecipeSection>(blankRecipe())
const recipeResetToken = ref(0)
const newCarId = ref<string | null>(null)

// Set after the card is first created (via ensureCard or onCreate); subsequent photo actions use this id.
const importedCard = ref<{ id: string; name: string; subtitle: string; collections: string[] } | null>(null)

const sectionOpen = reactive({ insp: true, notes: true, recipe: true })

// Form state
const saving = ref(false)
const error = ref('')
const nameRef = ref<HTMLInputElement | null>(null)

const nextNumber = computed(() => {
  const max = store.cards.reduce((m, c) => Math.max(m, c.catalogNumber), 0)
  return String(max + 1).padStart(3, '0')
})

const existingCollections = computed(() => store.allCollectionValues())
const existingTags = computed(() => store.allTagValues())

const filteredTags = computed(() => {
  const q = tagInput.value.toLowerCase().trim()
  const available = existingTags.value.filter(t => !selectedTags.value.includes(t))
  if (!q) return available
  return available.filter(t => t.toLowerCase().includes(q))
})

const canCreateTag = computed(() => {
  const q = tagInput.value.trim()
  if (!q || selectedTags.value.includes(q)) return false
  return !existingTags.value.some(t => t.toLowerCase() === q.toLowerCase())
})

watch(() => modal.newCardOpen, (open) => {
  document.body.style.overflow = open ? 'hidden' : ''
})

// Fires every time the modal is explicitly opened, even if it was already open.
watch(() => modal.newCardOpenCount, async () => {
  if (!modal.newCardOpen) return
  name.value = ''
  subtitle.value = ''
  selectedCollections.value = []
  collectionInput.value = ''
  selectedTags.value = []
  tagInput.value = ''
  inspirationBody.value = ''
  notesBody.value = ''
  inspirationFigurePath.value = null
  notesFigurePath.value = null
  recipe.value = blankRecipe()
  recipeResetToken.value++
  newCarId.value = null
  importedCard.value = null
  error.value = ''
  await nextTick()
  nameRef.value?.focus()
})

// Collections
function toggleCollection(c: string) {
  selectedCollections.value = selectedCollections.value.includes(c)
    ? selectedCollections.value.filter(x => x !== c)
    : [...selectedCollections.value, c]
}
function addCollectionFromInput() {
  const v = collectionInput.value.trim()
  if (v && !selectedCollections.value.includes(v)) selectedCollections.value.push(v)
  collectionInput.value = ''
}
function onCollectionKey(e: KeyboardEvent) {
  if (e.key === 'Enter') { e.preventDefault(); addCollectionFromInput() }
  if (e.key === 'Backspace' && !collectionInput.value) selectedCollections.value.pop()
}

// Tags
function addTag(tag: string) {
  const t = tag.trim()
  if (t && !selectedTags.value.includes(t)) selectedTags.value.push(t)
  tagInput.value = ''
}
function removeTag(tag: string) {
  selectedTags.value = selectedTags.value.filter(t => t !== tag)
}
function onTagKey(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault()
    const q = tagInput.value.trim()
    if (!q) return
    filteredTags.value.length > 0 ? addTag(filteredTags.value[0]) : canCreateTag.value && addTag(q)
  }
  if (e.key === 'Backspace' && !tagInput.value) selectedTags.value.pop()
}

// Photo count for UI feedback (only meaningful once the card is pre-created)
const photoCount = computed(() =>
  importedCard.value ? (store.byId(importedCard.value.id)?.images.length ?? 0) : 0
)

async function onCreate() {
  if (!name.value.trim() && !importedCard.value) { error.value = 'Name is required.'; return }
  saving.value = true
  error.value = ''
  try {
    if (importedCard.value) {
      // Card was pre-created by ensureCard — update it with the final form data.
      const existing = store.byId(importedCard.value.id)
      if (existing) {
        existing.name = name.value.trim() || existing.name
        existing.subtitle = subtitle.value.trim()
        existing.collections = selectedCollections.value
        existing.tags = selectedTags.value
        const insp = existing.sections.find(s => s.key === 'inspiration')
        if (insp && insp.type === 'text') { insp.body = inspirationBody.value.trim(); insp.figurePath = inspirationFigurePath.value ?? undefined }
        const notes = existing.sections.find(s => s.key === 'notes')
        if (notes && notes.type === 'text') { notes.body = notesBody.value.trim(); notes.figurePath = notesFigurePath.value ?? undefined }
        const rec = existing.sections.find(s => s.type === 'forza_recipe')
        if (rec && rec.type === 'forza_recipe') {
          rec.tuneName = recipe.value.tuneName.trim()
          rec.shareCode = recipe.value.shareCode.trim()
          rec.coreSpecs = { ...recipe.value.coreSpecs }
          rec.upgrades = JSON.parse(JSON.stringify(recipe.value.upgrades))
          rec.adjustments = JSON.parse(JSON.stringify(recipe.value.adjustments))
        }
        existing.carId = newCarId.value ?? undefined
        await store.save(existing.id)
      }
    } else {
      const card = await store.createNewCard({
        name: name.value.trim(),
        subtitle: subtitle.value.trim(),
        collections: selectedCollections.value,
        tags: selectedTags.value,
        inspirationBody: inspirationBody.value.trim(),
        inspirationFigurePath: inspirationFigurePath.value ?? undefined,
        notesBody: notesBody.value.trim(),
        notesFigurePath: notesFigurePath.value ?? undefined,
        tuneName: recipe.value.tuneName.trim(),
        shareCode: recipe.value.shareCode.trim(),
        coreSpecs: { ...recipe.value.coreSpecs },
        upgrades: JSON.parse(JSON.stringify(recipe.value.upgrades)),
        adjustments: JSON.parse(JSON.stringify(recipe.value.adjustments)),
        carId: newCarId.value ?? undefined,
      })
      for (const s of card.sections) {
        if (s.key === 'inspiration') s.defaultOpen = sectionOpen.insp ? undefined : false
        else if (s.key === 'notes') s.defaultOpen = sectionOpen.notes ? undefined : false
        else if (s.type === 'forza_recipe') s.defaultOpen = sectionOpen.recipe ? undefined : false
      }
      await store.save(card.id)
    }

    modal.closeNewCard()
    await nextTick()
    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' })
  } catch (e) {
    error.value = errMsg(e)
  } finally {
    saving.value = false
  }
}

async function onCancel() {
  if (importedCard.value) {
    const id = importedCard.value.id
    importedCard.value = null
    await store.deleteCard(id).catch(() => {})
  }
  modal.closeNewCard()
}
function onOverlay(e: MouseEvent) { if (e.target === e.currentTarget) onCancel() }
onUnmounted(() => { document.body.style.overflow = '' })
</script>

<template>
  <div class="nc-overlay" :class="{ open: modal.newCardOpen }" @click="onOverlay">
    <div class="card nc-modal-card">
      <button class="nc-close" aria-label="Cancel" @click="onCancel">×</button>

      <!-- card-meta: catalog number, collections, name, subtitle -->
      <div class="card-meta nc-card-meta">
        <div class="nc-meta-inner">
          <p class="card-number">
            CATALOG <span>NO. {{ nextNumber }}</span>
            <span
              v-for="c in selectedCollections" :key="c"
              class="collection-badge nc-badge"
            >{{ c }}<button class="nc-x" type="button" @click="toggleCollection(c)">×</button></span>
            <input
              class="nc-col-inline"
              v-model="collectionInput"
              placeholder="+ collection"
              @keydown="onCollectionKey"
              @blur="addCollectionFromInput"
            />
          </p>
          <div v-if="existingCollections.length" class="nc-col-picks">
            <button
              v-for="c in existingCollections" :key="c"
              class="nc-col-pick" :class="{ 'nc-col-pick--on': selectedCollections.includes(c) }"
              type="button"
              @click="toggleCollection(c)"
            >{{ c }}</button>
          </div>
          <input
            ref="nameRef"
            class="card-title nc-title-input"
            v-model="name"
            placeholder="Livery Name"
            @keydown.enter.prevent
          />
          <SubtitleEditor v-model="subtitle" />
        </div>
      </div>

      <!-- Photo zone — opens Photo Manager overlay -->
      <div class="nc-photo-zone">
        <button class="nc-add-photos-btn" type="button" @click="openPhotoManager">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
            <rect x="3" y="3" width="18" height="18" rx="2"/>
            <circle cx="8.5" cy="8.5" r="1.5"/>
            <path d="m21 15-5-5L5 21"/>
          </svg>
          {{ photoCount > 0 ? `${photoCount} photo${photoCount !== 1 ? 's' : ''} · Manage` : 'Add Photos' }}
        </button>
      </div>

      <!-- Tag cloud -->
      <div class="tag-cloud nc-tag-cloud">
        <span v-for="t in selectedTags" :key="t" class="tag nc-tag-sel">
          {{ t }}<button class="nc-x" type="button" @click="removeTag(t)">×</button>
        </span>
        <input
          class="nc-tag-input"
          v-model="tagInput"
          placeholder="Filter or add tags…"
          @keydown="onTagKey"
        />
        <button
          v-for="t in filteredTags" :key="t"
          class="nc-tag-opt"
          type="button"
          @click="addTag(t)"
        >{{ t }}</button>
        <button
          v-if="canCreateTag"
          class="nc-tag-opt nc-tag-new"
          type="button"
          @click="addTag(tagInput)"
        >+ "{{ tagInput.trim() }}"</button>
      </div>

      <!-- Inspiration — gutter-layout with figure upload + textarea -->
      <CollapsibleSection label="Inspiration" section-key="nc-insp" v-model:open="sectionOpen.insp">
        <div class="section-body gutter-layout">
          <div class="gutter-figure" :class="{ 'has-image': inspirationFigurePath }">
            <img
              v-if="inspirationFigurePath"
              class="gutter-figure-img"
              :src="inspirationFigurePath"
              @click="modal.openLightbox(inspirationFigurePath!)"
            />
            <span v-else class="gutter-figure-empty">Select image</span>
            <button class="change-image-btn" type="button" @click="openPickerForSection('insp')">
              {{ inspirationFigurePath ? 'Change Image' : 'Select Image' }}
            </button>

          </div>
          <textarea
            class="nc-textarea anecdote-text"
            v-model="inspirationBody"
            rows="4"
            placeholder="The why/how/when behind the build, told with some license."
          />
        </div>
      </CollapsibleSection>

      <!-- Design Notes — gutter-layout with figure upload + textarea -->
      <CollapsibleSection label="Design Notes" section-key="nc-notes" v-model:open="sectionOpen.notes">
        <div class="section-body gutter-layout">
          <div class="gutter-figure" :class="{ 'has-image': notesFigurePath }">
            <img
              v-if="notesFigurePath"
              class="gutter-figure-img"
              :src="notesFigurePath"
              @click="modal.openLightbox(notesFigurePath!)"
            />
            <span v-else class="gutter-figure-empty">Select image</span>
            <button class="change-image-btn" type="button" @click="openPickerForSection('notes')">
              {{ notesFigurePath ? 'Change Image' : 'Select Image' }}
            </button>
          </div>
          <textarea
            class="nc-textarea gutter-text"
            v-model="notesBody"
            rows="4"
            placeholder="Technique commentary — how it was built, material/layering choices."
          />
        </div>
      </CollapsibleSection>

      <!-- Tune / Build Parts — RecipeSection for full edit-mode parity -->
      <CollapsibleSection label="Tune / Build Parts" section-key="nc-recipe" v-model:open="sectionOpen.recipe">
        <RecipeSection
          :recipe="recipe"
          :initial-kit-open="true"
          :force-edit="true"
          :car-id="newCarId"
          :reset-token="recipeResetToken"
          @update:recipe="updated => Object.assign(recipe, updated)"
          @update:car-id="id => { newCarId = id }"
        />
      </CollapsibleSection>

      <!-- Footer -->
      <div class="nc-footer">
        <p v-if="error" class="nc-error">{{ error }}</p>
        <div class="nc-actions">
          <button class="nc-btn-cancel" type="button" @click="onCancel">Cancel</button>
          <button class="nc-btn-create" type="button" :disabled="saving" @click="onCreate">
            {{ saving ? 'Saving…' : 'Create Card →' }}
          </button>
        </div>
      </div>


    </div>
  </div>
</template>

<style scoped>
/* Textarea within gutter-layout — fills the text column */
.gutter-layout .nc-textarea {
  flex: 1;
  min-width: 0;
  resize: vertical;
  align-self: stretch;
}

/* Photo zone — "Add Photos" button */
.nc-photo-zone {
  padding: 10px 14px;
  border-bottom: 1px solid var(--panel-edge);
  display: flex;
  align-items: center;
}
.nc-add-photos-btn {
  display: flex;
  align-items: center;
  gap: 7px;
  font: 700 11px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  padding: 7px 14px;
  border-radius: 3px;
  border: 1px dashed var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well) 50%, transparent);
  color: var(--muted);
  cursor: pointer;
  transition: border-color .15s, color .15s, background .15s;
}
.nc-add-photos-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
}


</style>
