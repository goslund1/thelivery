<script setup lang="ts">
import { ref, computed, watch, nextTick, toRef } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useSlideshow } from '../composables/useSlideshow'
import { useNetworkQuality } from '../composables/useNetworkQuality'
import { api } from '../api'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const images = toRef(props.card, 'images')

// Full pool sorted by order — shown in edit mode thumb rail.
const poolSorted = computed(() => [...images.value].sort((a, b) => a.order - b.order))

const stageRef = ref<HTMLElement | null>(null)
const barRef = ref<HTMLElement | null>(null)
const toggleRef = ref<HTMLElement | null>(null)

const { srcFor } = useNetworkQuality()

// useSlideshow internally filters to included-only.
const { ordered, index, toggleIcon, toggleLabel, toggle, onThumb } = useSlideshow(
  images,
  stageRef,
  barRef,
  toggleRef,
)

const thumbsRef = ref<HTMLElement | null>(null)
const canLeft = ref(false)
const canRight = ref(false)

function updateArrows() {
  const t = thumbsRef.value
  if (!t) return
  const maxScroll = t.scrollWidth - t.clientWidth
  canLeft.value = t.scrollLeft > 4
  canRight.value = t.scrollLeft < maxScroll - 4
}

watch(index, async () => {
  await nextTick()
  const t = thumbsRef.value
  const active = t?.querySelector<HTMLElement>('.thumb.active')
  if (t && active) {
    const railLeft = t.scrollLeft
    const railRight = railLeft + t.clientWidth
    const thumbLeft = active.offsetLeft
    const thumbRight = thumbLeft + active.offsetWidth
    if (thumbLeft < railLeft) t.scrollLeft = thumbLeft - 10
    else if (thumbRight > railRight) t.scrollLeft = thumbRight - t.clientWidth + 10
  }
  updateArrows()
})

let dragFrom = -1
function onDragStart(i: number) { dragFrom = i }
function onDrop(i: number) {
  if (dragFrom >= 0 && dragFrom !== i) {
    store.reorderImages(props.card.id, dragFrom, i)
    ui.markCardDirty(props.card.id)
  }
  dragFrom = -1
}
function setLead(imageId: string) {
  store.setLeadImage(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}
function toggleIncluded(imageId: string) {
  store.toggleImageIncluded(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}

// ── Single-file / batch add ───────────────────────────────────────────────────
const addInputRef = ref<HTMLInputElement | null>(null)
const uploadProgress = ref<{ done: number; total: number } | null>(null)
const uploadResult = ref<{ added: number; failed: number } | null>(null)

async function runUpload(
  files: File[],
  cardCtx: { name: string; subtitle: string; collections: string[] },
  startIndex?: number,
) {
  let added = 0, failed = 0
  uploadProgress.value = { done: 0, total: files.length }
  try {
    for (let i = 0; i < files.length; i++) {
      try {
        const fileIndex = startIndex !== undefined ? startIndex + i : undefined
        const { path, thumbPath, stagePath } = await api.uploadImage(files[i], cardCtx, fileIndex)
        store.addImageToPool(props.card.id, path, thumbPath, stagePath)
        ui.markCardDirty(props.card.id)
        added++
      } catch (e) {
        console.warn(`[gallery] upload failed for "${files[i].name}":`, e)
        failed++
      }
      uploadProgress.value!.done++
    }
  } finally {
    uploadProgress.value = null
    uploadResult.value = { added, failed }
    setTimeout(() => { uploadResult.value = null }, 5000)
  }
}

async function onAddFile(e: Event) {
  const files = Array.from((e.target as HTMLInputElement).files ?? [])
  if (addInputRef.value) addInputRef.value.value = ''
  if (!files.length) return
  await runUpload(files, {
    name: props.card.name,
    subtitle: props.card.subtitle,
    collections: props.card.collections,
  })
}

// ── Folder import ─────────────────────────────────────────────────────────────
// Convention: folder name must start with FHX_ (e.g. FH5_Nissan_S13_Midnight)
const FOLDER_RE = /^(FH\s*\d+)[_\s-](.+)$/i

type FolderImport =
  | { stage: 'confirm';  files: File[]; folderName: string; fhTag: string; descriptor: string }
  | { stage: 'clarify';  files: File[]; folderName: string; clarifyFh: string; clarifyName: string }

const folderImport = ref<FolderImport | null>(null)
const folderInputRef = ref<HTMLInputElement | null>(null)

function onFolderSelected(e: Event) {
  const all = Array.from((e.target as HTMLInputElement).files ?? [])
  if (folderInputRef.value) folderInputRef.value.value = ''
  // Filter to images only, sort by relative path for consistent ordering
  const files = all
    .filter(f => f.type.startsWith('image/'))
    .sort((a, b) => a.webkitRelativePath.localeCompare(b.webkitRelativePath))
  if (!files.length) return

  const folderName = files[0].webkitRelativePath.split('/')[0]
  const match = FOLDER_RE.exec(folderName)
  if (match) {
    folderImport.value = {
      stage: 'confirm',
      files,
      folderName,
      fhTag: match[1].toUpperCase(),
      descriptor: match[2].replace(/_/g, ' '),
    }
  } else {
    folderImport.value = {
      stage: 'clarify',
      files,
      folderName,
      clarifyFh: '',
      clarifyName: folderName.replace(/_/g, ' '),
    }
  }
}

async function confirmFolderImport() {
  const fi = folderImport.value
  if (!fi) return
  const startIndex = props.card.images.length
  let cardCtx: { name: string; subtitle: string; collections: string[] }

  if (fi.stage === 'confirm') {
    // Use the folder-derived tag merged with the current card's collections
    const collections = props.card.collections.includes(fi.fhTag)
      ? props.card.collections
      : [fi.fhTag, ...props.card.collections]
    cardCtx = { name: props.card.name, subtitle: props.card.subtitle, collections }
  } else {
    const tag = fi.clarifyFh.trim().toUpperCase() || 'FHX'
    cardCtx = {
      name: fi.clarifyName.trim() || fi.folderName,
      subtitle: props.card.subtitle,
      collections: [tag, ...props.card.collections.filter(c => !c.toUpperCase().startsWith('FH'))],
    }
  }

  folderImport.value = null
  await runUpload(fi.files, cardCtx, startIndex)
}

function cancelFolderImport() {
  folderImport.value = null
}
</script>

<template>
  <div class="stage" ref="stageRef" :data-group="card.id">
    <div class="progress" ref="barRef"></div>
    <img
      v-for="(img, i) in ordered"
      :key="img.id"
      :src="srcFor(img, 'stage')"
      :class="{ active: i === index }"
      :data-index="i"
      @click="toggle"
    />
    <div class="stage-controls">
      <button class="play-toggle" ref="toggleRef" title="Pause or resume the slideshow" @click="toggle">
        <span class="icon" v-if="toggleIcon">{{ toggleIcon }}</span>
        <span class="label">{{ toggleLabel }}</span>
      </button>
      <button
        v-if="ordered[index]"
        class="stage-expand"
        title="View full resolution"
        @click.stop="ui.openLightbox(srcFor(ordered[index], 'stage'), ordered[index].path)"
      >⤢</button>
    </div>
  </div>

  <!-- Folder import banner lives ABOVE the rail so it spans full width -->
  <template v-if="ui.isEditing">
    <div v-if="folderImport" class="folder-import-banner">
      <template v-if="folderImport.stage === 'confirm'">
        <span class="fi-label">
          <strong>{{ folderImport.fhTag }}</strong> · {{ folderImport.descriptor }}
          <em>({{ folderImport.files.length }} photos)</em>
        </span>
        <button class="fi-btn fi-confirm" @click="confirmFolderImport">Import →</button>
        <button class="fi-btn fi-cancel" @click="cancelFolderImport">✕</button>
      </template>
      <template v-else>
        <span class="fi-label fi-warn">Folder name not recognised — clarify:</span>
        <input class="fi-input" v-model="folderImport.clarifyFh" placeholder="FH5 / FH6…" maxlength="6" />
        <input class="fi-input fi-input-wide" v-model="folderImport.clarifyName" placeholder="Card / folder name" />
        <span class="fi-count">{{ folderImport.files.length }} photos</span>
        <button class="fi-btn fi-confirm" :disabled="!folderImport.clarifyFh.trim()" @click="confirmFolderImport">Import →</button>
        <button class="fi-btn fi-cancel" @click="cancelFolderImport">✕</button>
      </template>
    </div>
    <div v-else-if="uploadResult" class="folder-import-banner" :class="{ 'fi-has-errors': uploadResult.failed > 0 }">
      <span class="fi-label">
        <template v-if="uploadResult.added > 0">✓ {{ uploadResult.added }} photo{{ uploadResult.added !== 1 ? 's' : '' }} added</template>
        <template v-if="uploadResult.failed > 0"> · {{ uploadResult.failed }} failed (check console — unsupported format?)</template>
      </span>
    </div>
  </template>

  <div class="thumb-rail">
    <div class="edge-arrow edge-arrow-left" :class="{ visible: canLeft }"><div class="tri"></div></div>
    <div class="thumbs" ref="thumbsRef" @scroll="updateArrows">

      <!-- Edit mode: thumb pool + add buttons -->
      <template v-if="ui.isEditing">
        <div
          v-for="(img, i) in poolSorted"
          :key="img.id"
          class="thumb"
          :class="{ active: ordered[index]?.id === img.id, excluded: img.included === false }"
          draggable="true"
          title="Drag to reorder"
          @click="onThumb(ordered.findIndex(o => o.id === img.id))"
          @dragstart="onDragStart(i)"
          @dragover.prevent
          @drop.prevent="onDrop(i)"
        >
          <img :src="img.thumbPath ?? img.path" />
          <div class="thumb-controls">
            <button
              class="thumb-ctrl thumb-ctrl-lead"
              :class="{ 'is-lead': img.order === 0 }"
              title="Set as lead image"
              @click.stop="setLead(img.id)"
            >★</button>
            <button
              class="thumb-ctrl thumb-ctrl-pool"
              :class="{ included: img.included !== false }"
              :title="img.included === false ? 'Add to slideshow' : 'Remove from slideshow'"
              @click.stop="toggleIncluded(img.id)"
            >●</button>
            <button
              class="thumb-ctrl thumb-ctrl-del"
              title="Remove from card"
              @click.stop="store.removeImage(card.id, img.id); ui.markCardDirty(card.id)"
            >✕</button>
          </div>
        </div>

        <!-- uploading progress -->
        <div v-if="uploadProgress" class="thumb thumb-add">
          <span class="thumb-add-progress">{{ uploadProgress.done }}/{{ uploadProgress.total }}</span>
        </div>

        <!-- + files  ⊞ folder -->
        <template v-else>
          <div class="thumb thumb-add" @click="addInputRef?.click()">
            <span class="thumb-add-icon">+</span>
            <span class="thumb-add-label" v-if="poolSorted.length === 0">Add photos</span>
            <input ref="addInputRef" type="file" accept="image/*" multiple style="display:none" @change="onAddFile" />
          </div>
          <div class="thumb thumb-add thumb-add-folder" title="Import a folder" @click="folderInputRef?.click()">
            <span class="thumb-add-icon">⊞</span>
            <input ref="folderInputRef" type="file" accept="image/*" webkitdirectory style="display:none" @change="onFolderSelected" />
          </div>
        </template>
      </template>

      <!-- View mode: included images only -->
      <template v-else>
        <div
          v-for="(img, i) in ordered"
          :key="img.id"
          class="thumb"
          :class="{ active: i === index }"
          :data-index="i"
          title="View this photo"
          @click="onThumb(i)"
        >
          <img :src="img.thumbPath ?? img.path" />
        </div>
      </template>

    </div>
    <div class="edge-arrow edge-arrow-right" :class="{ visible: canRight }"><div class="tri"></div></div>
  </div>
</template>

<style scoped>
.stage-expand {
  position: absolute;
  bottom: 10px;
  right: 10px;
  width: 28px;
  height: 28px;
  background: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  color: rgba(255, 255, 255, 0.7);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  opacity: 0;
  transition: opacity 0.2s ease, color 0.15s ease, border-color 0.15s ease;
}
.stage:hover .stage-expand {
  opacity: 1;
}
.stage-expand:hover {
  color: var(--gold);
  border-color: var(--gold);
}
.thumb.excluded {
  opacity: 0.35;
}
.thumb.excluded img {
  filter: grayscale(60%);
}

/* Three-button control row — top-left of each thumb in edit mode */
.thumb-controls {
  position: absolute;
  top: 3px;
  left: 3px;
  display: flex;
  gap: 2px;
  opacity: 0;
  z-index: 10;
  transition: opacity 0.15s ease;
}
.thumb:hover .thumb-controls {
  opacity: 1;
}
.thumb-ctrl {
  width: 16px;
  height: 16px;
  border-radius: 2px;
  border: none;
  background: rgba(0, 0, 0, 0.68);
  color: rgba(255, 255, 255, 0.55);
  font-size: 9px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: background 0.12s ease, color 0.12s ease;
}
/* ★ lead — gold when this is the lead image */
.thumb-ctrl-lead.is-lead,
.thumb-ctrl-lead:hover {
  color: var(--gold);
}
/* ● pool — gold when included in slideshow, dim when excluded */
.thumb-ctrl-pool.included {
  color: var(--gold);
}
.thumb-ctrl-pool:not(.included) {
  color: rgba(255, 255, 255, 0.3);
}
/* ✕ delete — red on hover */
.thumb-ctrl-del:hover {
  background: rgba(170, 30, 30, 0.9);
  color: #fff;
}
.thumb-add {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px dashed var(--panel-edge);
  background: transparent;
  cursor: pointer;
  min-width: 60px;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.thumb-add:hover {
  border-color: var(--gold);
  background: var(--gold-tint-04);
}
.thumb-add-icon {
  font-size: 20px;
  color: var(--steel);
  line-height: 1;
}
.thumb-add:hover .thumb-add-icon {
  color: var(--gold);
}
.thumb-add-group {
  display: contents;
}
.thumb-add-folder {
  font-size: 16px;
}
.folder-import-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  flex-wrap: wrap;
}
.fi-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--paper);
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fi-label em { color: var(--steel); font-style: normal; }
.fi-warn { color: var(--gold); }
.fi-input {
  background: var(--panel-bg);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 4px 8px;
  width: 72px;
  outline: none;
}
.fi-input:focus { border-color: var(--gold); }
.fi-input-wide { width: 180px; }
.fi-count {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--steel);
  white-space: nowrap;
}
.fi-btn {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  padding: 5px 12px;
  border-radius: 3px;
  cursor: pointer;
  border: none;
  white-space: nowrap;
}
.fi-confirm {
  background: var(--gold);
  color: var(--ink);
  font-weight: 700;
}
.fi-confirm:disabled { opacity: 0.4; cursor: default; }
.fi-cancel {
  background: transparent;
  border: 1px solid var(--panel-edge);
  color: var(--steel);
}
.fi-cancel:hover { color: var(--paper); }
.fi-has-errors { border-color: var(--gold); }
.thumb-add-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--steel);
  margin-top: 4px;
}
.thumb-add:hover .thumb-add-label {
  color: var(--gold);
}
.thumb-add-progress {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--gold);
  letter-spacing: 0.05em;
}
</style>
