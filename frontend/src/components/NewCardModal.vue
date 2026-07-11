<script setup lang="ts">
import { ref, reactive, computed, nextTick, watch, onUnmounted } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore, type PoolImage } from '../stores/modal'
import { useLiveriesStore } from '../stores/liveries'
import { api } from '../api'
import type { ForzaRecipeSection } from '../types'
import CollapsibleSection from './CollapsibleSection.vue'
import RecipeSection from './RecipeSection.vue'
import SubtitleEditor from './SubtitleEditor.vue'
import CarPicker from './CarPicker.vue'

const store = useCardsStore()
const modal = useModalStore()
const liveriesStore = useLiveriesStore()

// Fields
const name = ref('')
const subtitle = ref('')
const selectedCollections = ref<string[]>([])
const collectionInput = ref('')
const selectedTags = ref<string[]>([])
const tagInput = ref('')
const inspirationBody = ref('')
const notesBody = ref('')

// Section figure images — uploaded immediately, paths stored here
const inspirationFigurePath = ref<string | null>(null)
const notesFigurePath = ref<string | null>(null)

async function openFigurePicker(section: 'insp' | 'notes') {
  // Ensure a card exists in the DB so uploads have a card_id for proper association.
  let cardId = importedCard.value?.id ?? null
  if (!cardId) {
    saving.value = true
    error.value = ''
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
        carId: newCarId.value ?? undefined,
      })
      await store.save(card.id)
      importedCard.value = { id: card.id, name: card.name, subtitle: card.subtitle, collections: card.collections }
      cardId = card.id
    } catch (e) {
      error.value = (e as Error).message
      return
    } finally {
      saving.value = false
    }
  }

  // Auto-upload any staged files that haven't been uploaded yet so they appear in the pool.
  // This is how staged-but-not-yet-imported photos become available to figure pickers.
  const toUpload = staged.value.filter(s => !s.poolResult)
  if (toUpload.length > 0) {
    saving.value = true
    const cardCtx = importedCard.value!
    await Promise.all(toUpload.map(async s => {
      try {
        const result = await api.uploadImage(s.file, {
          name: cardCtx.name, subtitle: store.byId(cardCtx.id)?.subtitle ?? '',
          collections: cardCtx.collections, id: cardCtx.id,
        }, undefined, undefined, 'refimg')
        s.poolResult = { id: result.id!, path: result.path, thumbPath: result.thumbPath, stagePath: result.stagePath }
        pendingPool.value.push({ id: result.id, path: result.path, thumbPath: result.thumbPath, stagePath: result.stagePath })
        store.addImageToPool(cardCtx.id, result.path, result.thumbPath, result.stagePath, false, result.id)
      } catch (e) {
        console.warn('[figure-picker] pre-upload failed:', e)
      }
    }))
    saving.value = false
  }

  modal.openFigurePicker(cardId, () => pendingPool.value, (path, img) => {
    if (section === 'insp') inspirationFigurePath.value = path
    else notesFigurePath.value = path
    if (img) pendingPool.value.push(img)
  })
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
const imageRole = ref<'gallery' | 'refimg'>('gallery')

// Set after first successful import; subsequent rounds add to this card instead of creating a new one.
const importedCard = ref<{ id: string; name: string; subtitle: string; collections: string[] } | null>(null)
// Accumulates every upload (figure picks + batch) for the current new-card session.
// Passed to figure pickers so they show the full pool regardless of upload order.
const pendingPool = ref<PoolImage[]>([])
const showAddAnotherCar = ref(false)

// Livery name for batch import (required when photos are staged, default must be changed)
const liveryName = ref('Livery Name')
const liveryNameValid = computed(() =>
  liveryName.value.trim().length > 0 && liveryName.value.trim() !== 'Livery Name'
)

// Import progress log
interface ImportEntry { label: string; progress: number; status: 'uploading' | 'done' | 'error' }
const importing = ref(false)
const importLog = ref<ImportEntry[]>([])
const assessStatus = ref<'idle' | 'pending' | 'done' | 'error'>('idle')
const assessColors = ref<{ primary: string; secondary?: string } | null>(null)
const importFading = ref(false)
let importFadeTimer: ReturnType<typeof setTimeout> | null = null

const sectionOpen = reactive({ insp: true, notes: true, recipe: true })

// Upload staging (gallery images)
// poolResult is set after the file has been auto-uploaded for pool access
interface Staged { file: File; url: string; poolResult?: { id: number; path: string; thumbPath?: string; stagePath?: string } }
const staged = ref<Staged[]>([])
const activeStaged = ref(0)
const isDragOver = ref(false)

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
  imageRole.value = 'gallery'
  liveryName.value = 'Livery Name'
  importing.value = false
  importLog.value = []
  assessStatus.value = 'idle'
  assessColors.value = null
  importFading.value = false
  importedCard.value = null
  pendingPool.value = []
  showAddAnotherCar.value = false
  if (importFadeTimer) { clearTimeout(importFadeTimer); importFadeTimer = null }
  staged.value.forEach(s => URL.revokeObjectURL(s.url))
  staged.value = []
  activeStaged.value = 0
  isDragOver.value = false
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

// Gallery upload staging
const SUPPORTED = new Set(['image/jpeg', 'image/png', 'image/webp'])
function stageFiles(files: FileList | File[]) {
  for (const f of files) {
    if (!SUPPORTED.has(f.type)) continue
    staged.value.push({ file: f, url: URL.createObjectURL(f) })
  }
  activeStaged.value = staged.value.length - 1
}
function onDrop(e: DragEvent) {
  isDragOver.value = false
  if (e.dataTransfer?.files) stageFiles(e.dataTransfer.files)
}
function onFilePick(e: Event) {
  const files = (e.target as HTMLInputElement).files
  if (files) stageFiles(files)
  ;(e.target as HTMLInputElement).value = ''
}
function removeStaged(i: number) {
  URL.revokeObjectURL(staged.value[i].url)
  staged.value.splice(i, 1)
  activeStaged.value = Math.min(activeStaged.value, staged.value.length - 1)
}
function setFeature(i: number) {
  if (i === 0) return
  const moved = staged.value.splice(i, 1)[0]
  staged.value.unshift(moved)
  activeStaged.value = 0
}

function startAnotherCar() {
  staged.value.forEach(s => URL.revokeObjectURL(s.url))
  staged.value = []
  activeStaged.value = 0
  newCarId.value = null
  imageRole.value = 'gallery'
  liveryName.value = 'Livery Name'
  importing.value = false
  importLog.value = []
  assessStatus.value = 'idle'
  assessColors.value = null
  importFading.value = false
  showAddAnotherCar.value = false
  if (importFadeTimer) { clearTimeout(importFadeTimer); importFadeTimer = null }
}

async function onDone() {
  modal.closeNewCard()
  await nextTick()
  window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' })
}

// Create
async function onCreate() {
  if (!name.value.trim() && !importedCard.value) { error.value = 'Name is required.'; return }
  if (staged.value.length > 0 && !newCarId.value && imageRole.value !== 'refimg') { error.value = 'Select a car or +IMG before importing photos.'; return }
  if (staged.value.length > 0 && !liveryNameValid.value && imageRole.value !== 'refimg') { error.value = 'Enter a unique livery name (not the default).'; return }
  saving.value = true
  error.value = ''
  try {
    let cardCtx: { id: string; name: string; subtitle: string; collections: string[] }
    if (importedCard.value) {
      // Card was pre-created by the figure picker — update it with the final form data.
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
        importedCard.value = { id: existing.id, name: existing.name, subtitle: existing.subtitle, collections: existing.collections }
      }
      cardCtx = importedCard.value
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
      cardCtx = { id: card.id, name: card.name, subtitle: card.subtitle, collections: card.collections }
      importedCard.value = cardCtx
    }

    if (staged.value.length === 0) {
      modal.closeNewCard()
      await nextTick()
      window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' })
      return
    }

    // For gallery imports: create a livery to attach to every upload.
    // For refimg imports: no livery needed.
    // Skip livery creation if all staged files were already auto-uploaded for the pool.
    const isRefImg = imageRole.value === 'refimg'
    const hasNewUploads = staged.value.some(s => !s.poolResult)
    let liveryId: number | undefined
    if (!isRefImg && hasNewUploads && newCarId.value && liveryNameValid.value) {
      const livery = await liveriesStore.create({ carId: newCarId.value, name: liveryName.value.trim() })
      liveryId = livery.id
    }

    // Switch to import log view.
    importing.value = true
    importLog.value = staged.value.map(s => ({ label: s.file.name, progress: s.poolResult ? 100 : 0, status: s.poolResult ? 'done' as const : 'uploading' as const }))
    assessStatus.value = liveryId ? 'pending' : 'idle'

    let firstDone = false
    const uploads = staged.value.map((s, i) => {
      // Already uploaded for pool — mark done immediately, skip re-upload.
      if (s.poolResult) {
        importLog.value[i].progress = 100
        importLog.value[i].status = 'done'
        return Promise.resolve()
      }
      return api.uploadImageWithProgress(
        s.file,
        { name: cardCtx.name, subtitle: cardCtx.subtitle, collections: cardCtx.collections, id: cardCtx.id },
        { fileIndex: i, carId: newCarId.value ?? undefined, liveryId, imageRole: imageRole.value },
        (pct) => { importLog.value[i].progress = pct },
      ).then(result => {
        importLog.value[i].progress = 100
        importLog.value[i].status = 'done'
        pendingPool.value.push({ id: result.id, path: result.path, thumbPath: result.thumbPath, stagePath: result.stagePath })
        store.addImageToPool(cardCtx.id, result.path, result.thumbPath, result.stagePath, !isRefImg, result.id)
        if (!firstDone && liveryId) {
          firstDone = true
          api.assessLiveryColor(liveryId)
            .then(r => { assessStatus.value = 'done'; assessColors.value = { primary: r.primary, secondary: r.secondary } })
            .catch(() => { assessStatus.value = 'error' })
        }
      }).catch(() => { importLog.value[i].status = 'error' })
    })

    await Promise.all(uploads)
    await store.save(cardCtx.id)

    // Wait for assess to settle (it may still be in flight).
    const waitForAssess = () => new Promise<void>(resolve => {
      if (assessStatus.value !== 'pending') { resolve(); return }
      const id = setInterval(() => { if (assessStatus.value !== 'pending') { clearInterval(id); resolve() } }, 200)
    })
    await waitForAssess()

    // All done — fade log, then offer "add another car" instead of auto-closing.
    importFadeTimer = setTimeout(() => {
      importFading.value = true
      importFadeTimer = setTimeout(() => {
        importing.value = false
        showAddAnotherCar.value = true
      }, 700)
    }, 2000)
  } catch (e) {
    error.value = (e as Error).message
    importing.value = false
  } finally {
    saving.value = false
  }
}

function onLiveryFocus() { if (liveryName.value === 'Livery Name') liveryName.value = '' }
function onLiveryBlur() { if (!liveryName.value.trim()) liveryName.value = 'Livery Name' }
async function onCancel() {
  pendingPool.value = []
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

      <!-- Gallery / upload stage (16:9) -->
      <div
        class="nc-stage"
        :class="{ 'nc-stage--drag': isDragOver, 'nc-stage--filled': staged.length > 0 }"
        @dragenter.prevent="isDragOver = true"
        @dragover.prevent="isDragOver = true"
        @dragleave.prevent="isDragOver = false"
        @drop.prevent="onDrop"
      >
        <img
          v-if="staged.length"
          :src="staged[activeStaged]?.url"
          class="nc-stage-img"
          alt=""
        />
        <label class="nc-stage-prompt" :class="{ 'nc-stage-prompt--hidden': staged.length > 0 }">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2"/>
            <circle cx="8.5" cy="8.5" r="1.5"/>
            <path d="m21 15-5-5L5 21"/>
          </svg>
          <span>Drop photos here or <span class="nc-browse-link">browse</span></span>
          <input type="file" multiple accept="image/jpeg,image/png,image/webp" @change="onFilePick" />
        </label>
        <label v-if="staged.length" class="nc-stage-add-btn">
          + Add more
          <input type="file" multiple accept="image/jpeg,image/png,image/webp" @change="onFilePick" />
        </label>
      </div>

      <!-- Staged thumbnail strip -->
      <div v-if="staged.length" class="nc-thumb-strip">
        <div
          v-for="(s, i) in staged" :key="i"
          class="nc-thumb"
          :class="{ 'nc-thumb--active': i === activeStaged }"
          @click="activeStaged = i"
        >
          <img :src="s.url" alt="" />
          <button class="nc-thumb-x" type="button" @click.stop="removeStaged(i)">×</button>
          <button
            class="nc-thumb-star"
            :class="{ 'is-feature': i === 0 }"
            type="button"
            :title="i === 0 ? 'Feature image' : 'Set as feature image'"
            @click.stop="setFeature(i)"
          >★</button>
        </div>
      </div>

      <!-- Photo setup: car + livery name — shown when photos are staged -->
      <div v-if="staged.length" class="nc-photo-setup">
        <div class="nc-setup-row">
          <span class="nc-setup-label">Car</span>
          <template v-if="imageRole === 'refimg'">
            <span class="nc-refimg-chip">
              <span class="nc-refimg-badge">Img</span>
              <span class="nc-refimg-label">RefImg</span>
              <button class="nc-refimg-clear" type="button" @click="imageRole = 'gallery'">×</button>
            </span>
          </template>
          <CarPicker
            v-else
            :car-id="newCarId"
            :show-image-btn="true"
            @update:car-id="id => { newCarId = id; imageRole = 'gallery' }"
            @select-image="imageRole = 'refimg'; newCarId = null"
          />
        </div>
        <div v-if="imageRole !== 'refimg'" class="nc-setup-row">
          <span class="nc-setup-label">Livery</span>
          <input
            class="nc-livery-input"
            :class="{ 'nc-livery-input--default': !liveryNameValid }"
            v-model="liveryName"
            placeholder="Unique livery name…"
            @focus="onLiveryFocus"
            @blur="onLiveryBlur"
          />
        </div>
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
            <button class="change-image-btn" type="button" @click="openFigurePicker('insp')">
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
            <button class="change-image-btn" type="button" @click="openFigurePicker('notes')">
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
        <template v-if="showAddAnotherCar">
          <div class="nc-post-import">
            <span class="nc-post-import-msg">Import complete ✓</span>
            <div class="nc-post-import-actions">
              <button class="nc-btn-cancel" type="button" @click="onDone">Done</button>
              <button class="nc-btn-create" type="button" @click="startAnotherCar">+ Add another car</button>
            </div>
          </div>
        </template>
        <template v-else-if="importing">
          <!-- Import progress log -->
          <div class="nc-import-log" :class="{ 'nc-import-log--fading': importFading }">
            <div
              v-for="(entry, i) in importLog"
              :key="i"
              class="nc-import-row"
              :class="'nc-import-row--' + entry.status"
              :style="{ '--prog': entry.progress + '%' }"
            >
              <span class="nc-import-label">{{ entry.label }}</span>
              <span class="nc-import-status">
                {{ entry.status === 'uploading' ? entry.progress + '%' : entry.status === 'done' ? '✓' : '✗' }}
              </span>
            </div>
            <div v-if="assessStatus !== 'idle'" class="nc-import-row" :class="assessStatus === 'pending' ? 'nc-import-row--uploading' : assessStatus === 'done' ? 'nc-import-row--done' : 'nc-import-row--error'" :style="{ '--prog': assessStatus === 'pending' ? '60%' : '100%' }">
              <span class="nc-import-label">Color assess</span>
              <span class="nc-import-status">
                <template v-if="assessStatus === 'pending'">assessing…</template>
                <template v-else-if="assessStatus === 'done' && assessColors">{{ assessColors.primary }}<template v-if="assessColors.secondary"> / {{ assessColors.secondary }}</template></template>
                <template v-else>failed</template>
              </span>
            </div>
          </div>
        </template>
        <template v-else>
          <p v-if="error" class="nc-error">{{ error }}</p>
          <div class="nc-actions">
            <button class="nc-btn-cancel" type="button" @click="onCancel">Cancel</button>
            <button class="nc-btn-create" type="button" :disabled="saving" @click="onCreate">
              {{ saving ? 'Creating…' : staged.length ? 'Import →' : 'Create Card →' }}
            </button>
          </div>
        </template>
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

/* Photo setup — car + livery name row */
.nc-photo-setup {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well, #111) 40%, transparent);
}
.nc-setup-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.nc-setup-label {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
  min-width: 40px;
  padding-top: 6px;
  flex-shrink: 0;
}
.nc-livery-input {
  flex: 1;
  font: 12px/1 'JetBrains Mono', monospace;
  padding: 5px 8px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well) 60%, transparent);
  color: var(--fg);
  outline: none;
  transition: border-color .12s;
}
.nc-livery-input:focus { border-color: var(--accent); }
.nc-livery-input--default { color: var(--muted); border-style: dashed; }

.nc-refimg-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 6px 3px 4px;
  border-radius: 4px;
  border: 1px dashed var(--muted-light, #444);
  background: color-mix(in srgb, var(--panel-well, #1a1a1a) 60%, transparent);
}
.nc-refimg-badge {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  padding: 2px 5px;
  border-radius: 3px;
  background: var(--muted-light, #444);
  color: var(--text-muted, #aaa);
}
.nc-refimg-label {
  font: 12px/1.2 'Oswald', sans-serif;
  color: var(--text-primary, #e0e0e0);
}
.nc-refimg-clear {
  font: 14px/1 monospace;
  background: none;
  border: none;
  color: var(--text-muted, #888);
  cursor: pointer;
  padding: 0 2px;
}
.nc-refimg-clear:hover { color: var(--text-primary); }

/* Import log */
.nc-import-log {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 0 4px;
  opacity: 1;
  transition: opacity 0.7s ease;
}
.nc-import-log--fading { opacity: 0; }

.nc-import-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 5px 10px;
  border-radius: 3px;
  font: 11px/1 'JetBrains Mono', monospace;
  overflow: hidden;
}
.nc-import-row::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to right,
    color-mix(in srgb, var(--accent) 16%, transparent) var(--prog, 0%),
    transparent var(--prog, 0%)
  );
  transition: background 0.3s ease;
  pointer-events: none;
}
.nc-import-label {
  color: var(--muted);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  position: relative;
}
.nc-import-status {
  flex-shrink: 0;
  position: relative;
}
.nc-import-row--uploading .nc-import-status { color: var(--muted); }
.nc-import-row--done .nc-import-label,
.nc-import-row--done .nc-import-status { color: var(--accent); }
.nc-import-row--error .nc-import-label,
.nc-import-row--error .nc-import-status { color: #c94444; }

/* Post-import — done / add another car */
.nc-post-import {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 0 4px;
}
.nc-post-import-msg {
  font: 11px/1 'JetBrains Mono', monospace;
  color: var(--accent);
  letter-spacing: .06em;
}
.nc-post-import-actions {
  display: flex;
  gap: 8px;
}


</style>
