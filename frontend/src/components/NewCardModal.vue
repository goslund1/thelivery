<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { api } from '../api'

const store = useCardsStore()
const ui = useUiStore()

// Fields
const name = ref('')
const subtitle = ref('')
const selectedCollections = ref<string[]>([])
const collectionInput = ref('')
const selectedTags = ref<string[]>([])
const tagInput = ref('')
const inspirationBody = ref('')
const notesBody = ref('')
const tuneName = ref('')
const shareCode = ref('')
const CORE_SPEC_KEYS = ['Drivetrain', 'Engine', 'Transmission', 'Tires', 'Suspension']
const coreSpecs = ref<Record<string, string>>(Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, ''])))

// Upload staging
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

watch(() => ui.newCardOpen, async (open) => {
  if (!open) return
  name.value = ''
  subtitle.value = ''
  selectedCollections.value = []
  collectionInput.value = ''
  selectedTags.value = []
  tagInput.value = ''
  inspirationBody.value = ''
  notesBody.value = ''
  tuneName.value = ''
  shareCode.value = ''
  coreSpecs.value = Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, '']))
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

// Upload staging
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

// Create
async function onCreate() {
  if (!name.value.trim()) { error.value = 'Name is required.'; return }
  saving.value = true
  error.value = ''
  try {
    const card = await store.createNewCard({
      name: name.value.trim(),
      subtitle: subtitle.value.trim(),
      collections: selectedCollections.value,
      tags: selectedTags.value,
      inspirationBody: inspirationBody.value.trim(),
      notesBody: notesBody.value.trim(),
      tuneName: tuneName.value.trim(),
      shareCode: shareCode.value.trim(),
      coreSpecs: Object.fromEntries(
        Object.entries(coreSpecs.value).filter(([, v]) => v.trim())
      ),
    })
    for (let i = 0; i < staged.value.length; i++) {
      const result = await api.uploadImage(staged.value[i].file, card, i)
      store.addImageToPool(card.id, result.path, result.thumbPath, result.stagePath)
    }
    if (staged.value.length > 0) await store.save(card.id)
    staged.value.forEach(s => URL.revokeObjectURL(s.url))
    ui.closeNewCard()
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
  <div class="nc-overlay" :class="{ open: ui.newCardOpen }" @click="onOverlay">
    <div class="card nc-modal-card">
      <button class="nc-close" aria-label="Cancel" @click="onCancel">×</button>

      <!-- ── card-meta: catalog number, collections, name, subtitle ── -->
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
          <!-- existing collection quick-picks -->
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
          <input
            class="card-sub nc-sub-input"
            v-model="subtitle"
            placeholder="Make/Model (game-accurate) · Base Coat or Technique · Style · Rims"
          />
        </div>
      </div>

      <!-- ── Gallery / upload stage (16:9) ── -->
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

      <!-- ── Tag cloud ── -->
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

      <!-- ── Sections ── -->
      <div class="nc-section">
        <div class="nc-section-head">Inspiration</div>
        <div class="section-body">
          <textarea
            class="nc-textarea"
            v-model="inspirationBody"
            rows="4"
            placeholder="This is where the creative-fiction origin story lives — the why/how/when behind the build, told with some license. Doesn't need to be literally true, just true to the car."
          />
        </div>
      </div>

      <div class="nc-section">
        <div class="nc-section-head">Design Notes</div>
        <div class="section-body">
          <textarea
            class="nc-textarea"
            v-model="notesBody"
            rows="4"
            placeholder="This is where technique commentary lives — how it was actually built in the livery editor, material/layering choices, anything the tools fought you on."
          />
        </div>
      </div>

      <div class="nc-section">
        <div class="nc-section-head">Tune / Build Parts</div>
        <div class="section-body">
          <div class="nc-recipe-row">
            <div class="nc-recipe-field">
              <label class="nc-recipe-label">Tune Name</label>
              <input class="nc-recipe-input" v-model="tuneName" placeholder="Tune Name (the name it's saved under in-game)" />
            </div>
            <div class="nc-recipe-field nc-recipe-field--narrow">
              <label class="nc-recipe-label">Share Code</label>
              <input class="nc-recipe-input" v-model="shareCode" placeholder="000 000 000" />
            </div>
          </div>
          <div class="nc-specs-grid">
            <div v-for="key in CORE_SPEC_KEYS" :key="key" class="nc-spec">
              <label class="nc-recipe-label">{{ key }}</label>
              <input class="nc-recipe-input" v-model="coreSpecs[key]" :placeholder="`e.g. ${key === 'Drivetrain' ? 'AWD' : key === 'Tires' ? 'Rally' : 'stock'}`" />
            </div>
          </div>
          <p class="nc-stub-hint">
            <span>Build parts (upgrades installed) → fill in after recon.</span>
          </p>
        </div>
      </div>

      <!-- ── Footer ── -->
      <div class="nc-footer">
        <p v-if="error" class="nc-error">{{ error }}</p>
        <div class="nc-actions">
          <button class="nc-btn-cancel" type="button" @click="onCancel">Cancel</button>
          <button class="nc-btn-create" type="button" :disabled="saving" @click="onCreate">
            {{ saving ? 'Creating…' : 'Create Card →' }}
          </button>
        </div>
      </div>

    </div>
  </div>
</template>

