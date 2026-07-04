<script setup lang="ts">
import { ref, computed, watch, nextTick, toRef } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useSlideshow } from '../composables/useSlideshow'
import { useNetworkQuality } from '../composables/useNetworkQuality'
import { api } from '../api'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()
const modal = useModalStore()

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
    // In edit mode, the add panel covers the right 90px of the rail. Use that as
    // the effective right boundary so the active thumb always lands in the clear
    // zone — the gradient then shows the next thumb peeking in as a preview.
    const reservedRight = ui.isEditing ? 90 : 0
    const railLeft = t.scrollLeft
    const railRight = railLeft + t.clientWidth - reservedRight
    const thumbLeft = active.offsetLeft
    const thumbRight = thumbLeft + active.offsetWidth
    if (thumbLeft < railLeft) t.scrollLeft = thumbLeft - 10
    else if (thumbRight > railRight) t.scrollLeft = thumbRight - t.clientWidth + reservedRight + 10
  }
  updateArrows()
})

const dragFrom = ref(-1)
const dropIdx = ref(-1)
function onDragStart(i: number) { dragFrom.value = i; dropIdx.value = -1; onThumb(index.value) }
function onRailTouch() { if (ui.isEditing) onThumb(index.value) }
function onDragOver(i: number) { if (dragFrom.value >= 0) dropIdx.value = i }
function onDragEnd() { dragFrom.value = -1; dropIdx.value = -1 }
function onDrop(i: number) {
  if (dragFrom.value >= 0 && dragFrom.value !== i) {
    store.reorderImages(props.card.id, dragFrom.value, i)
    ui.markCardDirty(props.card.id)
  }
  dragFrom.value = -1
  dropIdx.value = -1
}
function setLead(imageId: string) {
  store.setLeadImage(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}
function toggleIncluded(imageId: string) {
  store.toggleImageIncluded(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}

// ── Upload ────────────────────────────────────────────────────────────────────
const uploadProgress = ref<{ done: number; total: number } | null>(null)
const pendingDeleteId = ref<string | null>(null)
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

// ── Folder import ─────────────────────────────────────────────────────────────
// Convention: folder name must start with FHX_ (e.g. FH5_Nissan_S13_Midnight)
const FOLDER_RE = /^(FH\s*\d+)[_\s-](.+)$/i

type FolderImport =
  | { stage: 'confirm';  files: File[]; folderName: string; fhTag: string; descriptor: string }
  | { stage: 'clarify';  files: File[]; folderName: string; clarifyFh: string; clarifyName: string }

const folderImport = ref<FolderImport | null>(null)
const folderInputRef = ref<HTMLInputElement | null>(null)

const SUPPORTED_TYPES = new Set(['image/jpeg', 'image/jpg', 'image/png', 'image/webp', 'image/gif'])
const HEIC_EXTS = /\.(heic|heif|avif)$/i

function onFolderSelected(e: Event) {
  const all = Array.from((e.target as HTMLInputElement).files ?? [])
  if (folderInputRef.value) folderInputRef.value.value = ''
  // Only JPEG/PNG/WebP — HEIC (iPhone default) not supported by the backend decoder
  const files = all
    .filter(f => SUPPORTED_TYPES.has(f.type) || (!HEIC_EXTS.test(f.name) && f.type.startsWith('image/')))
    .sort((a, b) => (a.webkitRelativePath || a.name).localeCompare(b.webkitRelativePath || b.name))
  const skipped = all.filter(f => !files.includes(f) && (f.type.startsWith('image/') || /\.(heic|heif|avif)$/i.test(f.name)))
  if (skipped.length) console.warn(`[gallery] skipped ${skipped.length} unsupported file(s) (HEIC/HEIF/AVIF). Export as JPEG from Photos first.`)
  if (!files.length) return

  // webkitRelativePath is only populated when a whole folder is selected.
  // Individual file picks won't have it, so fall back to clarify pre-filled from the card.
  const folderName = files[0].webkitRelativePath?.split('/')[0] ?? ''
  if (folderName) {
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
      const existingFh = props.card.collections.find(c => /^FH\d+$/i.test(c.trim())) ?? ''
      if (existingFh) {
        folderImport.value = {
          stage: 'confirm',
          files,
          folderName,
          fhTag: existingFh.toUpperCase(),
          descriptor: props.card.name,
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
  } else {
    const existingFh = props.card.collections.find(c => /^FH\d+$/i.test(c.trim())) ?? ''
    if (existingFh) {
      folderImport.value = {
        stage: 'confirm',
        files,
        folderName: '',
        fhTag: existingFh.toUpperCase(),
        descriptor: props.card.name,
      }
    } else {
      folderImport.value = {
        stage: 'clarify',
        files,
        folderName: '',
        clarifyFh: '',
        clarifyName: props.card.name,
      }
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
      :alt="img.alt ?? ''"
      :class="{ active: i === index }"
      :data-index="i"
      @click="toggle"
    />
    <div class="stage-controls">
      <button class="play-toggle" ref="toggleRef" title="Pause or resume the slideshow" @click="toggle">
        <span class="icon" v-if="toggleIcon">{{ toggleIcon }}</span>
        <span class="label">{{ toggleLabel }}</span>
      </button>
    </div>
    <button
      v-if="ordered[index]"
      class="stage-expand"
      title="View full resolution"
      @click.stop="modal.openLightbox(
        srcFor(ordered[index], 'stage'),
        ordered[index].path,
        ordered.map(img => ({ display: srcFor(img, 'stage'), original: img.path })),
        index
      )"
    >⤢</button>
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
        <template v-if="uploadResult.failed > 0"> · {{ uploadResult.failed }} failed — export as JPEG from Photos</template>
      </span>
    </div>
  </template>

  <div class="thumb-rail" :class="{ 'thumb-rail-editing': ui.isEditing }">
    <div class="edge-arrow edge-arrow-left" :class="{ visible: canLeft }"><div class="tri"></div></div>
    <div class="thumbs" :class="{ 'thumbs-editing': ui.isEditing }" ref="thumbsRef" @scroll="updateArrows" @touchstart.passive="onRailTouch" @wheel.passive="onRailTouch">

      <!-- Edit mode: image pool only (add button overlays right side) -->
      <template v-if="ui.isEditing">
        <template v-for="(img, i) in poolSorted" :key="img.id">
        <div v-if="dragFrom >= 0 && dropIdx === i && dragFrom !== i" class="thumb-drop-indicator" />
        <div
          class="thumb"
          :class="{ active: ordered[index]?.id === img.id, excluded: img.included === false, 'thumb-dragging': dragFrom === i }"
          draggable="true"
          title="Drag to reorder"
          @click="onThumb(ordered.findIndex(o => o.id === img.id))"
          @dragstart="onDragStart(i)"
          @dragover.prevent="onDragOver(i)"
          @dragend="onDragEnd"
          @drop.prevent="onDrop(i)"
        >
          <img :src="img.thumbPath ?? img.path" />
          <div class="thumb-controls">
            <div class="thumb-ctrl-left">
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
            </div>
            <button
              class="thumb-ctrl thumb-ctrl-del"
              title="Remove from card"
              @click.stop="pendingDeleteId = img.id"
            >✕</button>
          </div>
          <div v-if="pendingDeleteId === img.id" class="thumb-delete-confirm" @click.stop>
            <span>Remove?</span>
            <button @click.stop="store.removeImage(card.id, img.id); ui.markCardDirty(card.id); pendingDeleteId = null">Yes</button>
            <button @click.stop="pendingDeleteId = null">No</button>
          </div>
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

    <!-- Persistent add button — overlays right side of rail with a fade -->
    <div
      v-if="ui.isEditing"
      class="thumb-add-panel"
      :title="poolSorted.length > 0 ? 'Manage photos' : 'Add photos'"
      @click="poolSorted.length > 0 ? modal.openGalleryManager(card.id) : folderInputRef?.click()"
    >
      <span class="thumb-add-icon">
        <span v-if="uploadProgress" class="thumb-add-progress">{{ uploadProgress.done }}/{{ uploadProgress.total }}</span>
        <svg v-else width="24" height="17" viewBox="0 0 430.393 293.602" fill="none" stroke="currentColor" stroke-miterlimit="10">
          <rect x="4.065"   y="15.863"  width="192.391" height="122.877" stroke-width="7"/>
          <rect x="224.793" y="15.863"  width="192.391" height="122.877" stroke-width="7"/>
          <rect x="224.793" y="167.225" width="192.391" height="122.877" stroke-width="7"/>
          <rect x="3.5"     y="167.225" width="192.391" height="122.877" stroke-width="7"/>
          <rect x="343.282" y="11.182"  width="77.323"  height="77.323"  stroke="none" fill="var(--panel)"/>
          <line x1="381.012" y1="0"       x2="381.012" y2="98.761"   stroke-width="25"/>
          <line x1="331.631" y1="49.381"  x2="430.393" y2="49.381"   stroke-width="25"/>
        </svg>
      </span>
      <input ref="folderInputRef" type="file" accept="image/jpeg,image/png,image/webp" webkitdirectory multiple style="display:none" @change="onFolderSelected" />
    </div>
  </div>
</template>

<style scoped>
.stage-expand {
  position: absolute;
  top: 10px;
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

/* Thumb control row — spans top of each thumb in edit mode */
.thumb-controls {
  position: absolute;
  top: 3px;
  left: 3px;
  right: 3px;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  opacity: 0;
  z-index: 10;
  transition: opacity 0.15s ease;
}
.thumb:hover .thumb-controls {
  opacity: 1;
}
.thumb-ctrl-left {
  display: flex;
  gap: 2px;
}
.thumb-ctrl {
  width: 19px;
  height: 19px;
  border-radius: 3px;
  border: none;
  background: rgba(0, 0, 0, 0.68);
  color: rgba(255, 255, 255, 0.55);
  font-size: 10px;
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
/* Excluded thumbs: reverse contrast on controls — light bg, dark icons */
.thumb.excluded .thumb-ctrl {
  background: rgba(220, 220, 220, 0.75);
  color: rgba(0, 0, 0, 0.55);
}
.thumb.excluded .thumb-ctrl-lead.is-lead,
.thumb.excluded .thumb-ctrl-lead:hover {
  color: var(--gold);
}
.thumb.excluded .thumb-ctrl-pool.included {
  color: var(--gold);
}
.thumb.excluded .thumb-ctrl-pool:not(.included) {
  color: rgba(0, 0, 0, 0.3);
}
.thumb.excluded .thumb-ctrl-del:hover {
  background: rgba(170, 30, 30, 0.85);
  color: #fff;
}
.thumb-delete-confirm {
  position: absolute;
  inset: 0;
  background: rgba(0,0,0,0.82);
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  align-content: center;
  justify-content: center;
  row-gap: 5px;
  column-gap: 5px;
  border-radius: 3px;
  font-size: 11px;
  color: #fff;
  z-index: 15;
}
.thumb-delete-confirm span {
  width: 100%;
  text-align: center;
  color: var(--gold);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.04em;
}
.thumb-delete-confirm button {
  border-radius: 3px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  padding: 3px 10px;
  cursor: pointer;
  transition: background .15s ease, border-color .15s ease, box-shadow .15s ease;
}
.thumb-delete-confirm button:first-of-type {
  background: color-mix(in srgb, var(--danger) 30%, transparent);
  color: var(--danger-bright);
  border: 1px solid var(--danger);
}
.thumb-delete-confirm button:first-of-type:hover {
  background: color-mix(in srgb, var(--danger) 55%, transparent);
  border-color: var(--danger-bright);
  box-shadow: 0 0 10px color-mix(in srgb, var(--danger-bright) 50%, transparent);
}
.thumb-delete-confirm button:last-of-type {
  background: color-mix(in srgb, var(--gold) 20%, transparent);
  color: var(--gold);
  border: 1px solid color-mix(in srgb, var(--gold) 50%, transparent);
}
.thumb-delete-confirm button:last-of-type:hover {
  background: color-mix(in srgb, var(--gold) 35%, transparent);
  border-color: var(--gold);
  box-shadow: 0 0 10px color-mix(in srgb, var(--gold) 40%, transparent);
}
.thumb-dragging {
  opacity: 0.3;
}
.thumb-drop-indicator {
  width: 3px;
  height: 60px;
  flex: 0 0 auto;
  background: var(--gold);
  border-radius: 2px;
  box-shadow: 0 0 8px rgba(201,162,39,0.7);
}
/* 20px fade zone + 60px icon + 10px right pad = 90px total
   gradient covers only the fade zone; icon has solid background */
.thumbs-editing {
  padding-right: 90px;
}
.thumb-add-panel {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 90px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 14px 10px 24px 0;
  background: linear-gradient(to right, transparent, var(--panel) 20px);
  cursor: pointer;
  z-index: 5;
}
.thumb-add-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 60px;
  height: 60px;
  background: var(--panel);
  border: 1px dashed rgba(140, 140, 140, 0.5);
  border-radius: 2px;
  color: var(--steel);
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}
.thumb-add-panel:hover .thumb-add-icon {
  border-color: var(--gold);
  color: var(--gold);
  background: var(--gold-tint-04);
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
/* In edit mode, chevron sits over the add panel's fade zone. background:none so
   the arrow's own gradient doesn't extend the fade leftward on canRight changes. */
.thumb-rail-editing .edge-arrow-right {
  right: 63px;
  z-index: 6;
  background: none;
}
.thumb-add-progress {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--gold);
  letter-spacing: 0.05em;
  text-align: center;
}
</style>
