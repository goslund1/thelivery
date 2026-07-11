<script setup lang="ts">
import { ref, reactive, computed, nextTick, watch, onUnmounted } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
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
const figureSaving = ref(false)
const figureError = ref('')

// Inline folder-name prompt (shown when card name is empty and user tries to upload a figure)
const folderPromptOpen = ref(false)
const folderPromptValue = ref('')
let folderPromptResolve: ((v: string | null) => void) | null = null

async function promptFolderName(): Promise<string | null> {
  return new Promise(resolve => {
    folderPromptValue.value = ''
    folderPromptOpen.value = true
    folderPromptResolve = resolve
  })
}
function confirmFolderName() {
  const v = folderPromptValue.value.trim()
  folderPromptOpen.value = false
  folderPromptResolve?.(v || null)
  folderPromptResolve = null
}
function cancelFolderName() {
  folderPromptOpen.value = false
  folderPromptResolve?.(null)
  folderPromptResolve = null
}

async function onFigureFilePick(section: 'insp' | 'notes', e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  ;(e.target as HTMLInputElement).value = ''
  if (!file) return

  const cardName = name.value.trim()
  const uploadCtxName = cardName || await promptFolderName()
  if (!uploadCtxName) return

  figureSaving.value = true
  figureError.value = ''
  try {
    const { path } = await api.uploadImage(file, {
      name: uploadCtxName,
      subtitle: subtitle.value.trim(),
      collections: selectedCollections.value,
    })
    if (section === 'insp') inspirationFigurePath.value = path
    else notesFigurePath.value = path
  } catch (err) {
    figureError.value = (err as Error).message
  } finally {
    figureSaving.value = false
  }
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

// Set after first successful import; subsequent rounds add to this card instead of creating a new one.
const importedCard = ref<{ id: string; name: string; subtitle: string; collections: string[] } | null>(null)
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
interface Staged { file: File; url: string }
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

watch(() => modal.newCardOpen, async (open) => {
  document.body.style.overflow = open ? 'hidden' : ''
  if (!open) return
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
  figureSaving.value = false
  figureError.value = ''
  recipe.value = blankRecipe()
  recipeResetToken.value++
  newCarId.value = null
  liveryName.value = 'Livery Name'
  importing.value = false
  importLog.value = []
  assessStatus.value = 'idle'
  assessColors.value = null
  importFading.value = false
  importedCard.value = null
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
  if (!name.value.trim()) { error.value = 'Name is required.'; return }
  if (staged.value.length > 0 && !newCarId.value) { error.value = 'Select a car before importing photos.'; return }
  if (staged.value.length > 0 && !liveryNameValid.value) { error.value = 'Enter a unique livery name (not the default).'; return }
  saving.value = true
  error.value = ''
  try {
    let cardCtx: { id: string; name: string; subtitle: string; collections: string[] }

    if (importedCard.value) {
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

    // Create livery first so we have an ID to attach to every upload.
    const livery = await liveriesStore.create({ carId: newCarId.value!, name: liveryName.value.trim() })
    const liveryId = livery.id

    // Switch to import log view.
    importing.value = true
    importLog.value = staged.value.map(s => ({ label: s.file.name, progress: 0, status: 'uploading' as const }))
    assessStatus.value = 'pending'

    let firstDone = false
    const uploads = staged.value.map((s, i) =>
      api.uploadImageWithProgress(
        s.file,
        { name: cardCtx.name, subtitle: cardCtx.subtitle, collections: cardCtx.collections, id: cardCtx.id },
        { fileIndex: i, carId: newCarId.value ?? undefined, liveryId },
        (pct) => { importLog.value[i].progress = pct },
      ).then(result => {
        importLog.value[i].progress = 100
        importLog.value[i].status = 'done'
        store.addImageToPool(cardCtx.id, result.path, result.thumbPath, result.stagePath, true, result.id)
        // Trigger assess once, after first successful upload with a livery attached.
        if (!firstDone && liveryId) {
          firstDone = true
          api.assessLiveryColor(liveryId)
            .then(r => { assessStatus.value = 'done'; assessColors.value = { primary: r.primary, secondary: r.secondary } })
            .catch(() => { assessStatus.value = 'error' })
        }
      }).catch(() => { importLog.value[i].status = 'error' })
    )

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
function onCancel() { modal.closeNewCard() }
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
          <CarPicker :car-id="newCarId" @update:car-id="id => { newCarId = id }" />
        </div>
        <div class="nc-setup-row">
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
            <label class="change-image-btn" :class="{ 'nc-figure-saving': figureSaving }">
              {{ inspirationFigurePath ? 'Change Image' : 'Select Image' }}
              <input type="file" style="display:none" accept="image/*" :disabled="figureSaving" @change="onFigureFilePick('insp', $event)" />
            </label>
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
            <label class="change-image-btn" :class="{ 'nc-figure-saving': figureSaving }">
              {{ notesFigurePath ? 'Change Image' : 'Select Image' }}
              <input type="file" style="display:none" accept="image/*" :disabled="figureSaving" @change="onFigureFilePick('notes', $event)" />
            </label>
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
          <p v-if="figureError" class="nc-error">{{ figureError }}</p>
          <p v-if="error" class="nc-error">{{ error }}</p>
          <div class="nc-actions">
            <button class="nc-btn-cancel" type="button" @click="onCancel">Cancel</button>
            <button class="nc-btn-create" type="button" :disabled="saving" @click="onCreate">
              {{ saving ? 'Creating…' : staged.length ? 'Import →' : 'Create Card →' }}
            </button>
          </div>
        </template>
      </div>

      <!-- Inline folder-name prompt (shown when no card name is set at figure upload time) -->
      <div v-if="folderPromptOpen" class="nc-folder-prompt" @click.self="cancelFolderName">
        <div class="nc-folder-prompt-inner">
          <p class="nc-folder-prompt-label">Enter a folder name for this image</p>
          <p class="nc-folder-prompt-sub">Or enter the Livery Name above first and it'll be used automatically.</p>
          <input
            class="nc-folder-prompt-input"
            v-model="folderPromptValue"
            placeholder="e.g. Dragon Livery"
            @keydown.enter="confirmFolderName"
            @keydown.escape="cancelFolderName"
          />
          <div class="nc-folder-prompt-btns">
            <button class="nc-btn-cancel" type="button" @click="cancelFolderName">Cancel</button>
            <button class="nc-btn-create" type="button" @click="confirmFolderName">Use This Name →</button>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<style scoped>
.nc-figure-saving {
  opacity: 0.5;
  pointer-events: none;
}

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

/* Folder-name prompt — fixed to viewport, not the modal card */
.nc-folder-prompt {
  position: fixed;
  inset: 0;
  z-index: 1300;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
}
.nc-folder-prompt-inner {
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  padding: 22px 24px 20px;
  max-width: 360px;
  width: 90vw;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.nc-folder-prompt-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .08em;
  color: var(--fg);
  padding-bottom: 10px;
  border-bottom: 1px solid var(--glass-border);
}
.nc-folder-prompt-sub {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--muted);
  line-height: 1.6;
}
.nc-folder-prompt-input {
  width: 100%;
  box-sizing: border-box;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  background: color-mix(in srgb, var(--glass-bg) 60%, transparent);
  border: 1px solid var(--glass-border);
  border-radius: 4px;
  color: var(--fg);
  padding: 9px 12px;
}
.nc-folder-prompt-input:focus {
  outline: none;
  border-color: var(--accent);
}
.nc-folder-prompt-btns {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>
