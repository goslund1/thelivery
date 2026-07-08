<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useCardsStore } from '../stores/cards'
import { api } from '../api'
import PhotoDetail from './PhotoDetail.vue'

const ui = useUiStore()
const modal = useModalStore()
const store = useCardsStore()

const ctx = computed(() => modal.imagePicker)
const card = computed(() => (ctx.value ? store.byId(ctx.value.cardId) : undefined))
const gallery = computed(() => [...(card.value?.images ?? [])].sort((a, b) => a.order - b.order))
const isManage = computed(() => !!ctx.value && !ctx.value.sectionKey)

// ── Pick mode ────────────────────────────────────────────────────────────────
function pick(path: string) {
  const c = ctx.value
  if (!c?.sectionKey) return
  store.setFigure(c.cardId, c.sectionKey, path)
  ui.markCardDirty(c.cardId)
  modal.closeImagePicker()
}

async function onPickUpload(e: Event) {
  const c = ctx.value
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!c?.sectionKey || !file) return
  const cv = card.value
  const result = await api.uploadImage(file, {
    name: cv?.name ?? '',
    subtitle: cv?.subtitle ?? '',
    collections: cv?.collections ?? [],
    id: c.cardId,
  })
  store.setFigure(c.cardId, c.sectionKey, result.path)
  store.addImageToPool(c.cardId, result.path, result.thumbPath, result.stagePath, false, result.id)
  ui.markCardDirty(c.cardId)
  modal.closeImagePicker()
}

// ── Manage mode — selection ───────────────────────────────────────────────────
const selectedIds = ref<Set<number>>(new Set())
const lastClickedIndex = ref(-1)

function onThumbClick(e: MouseEvent, imageId: number, index: number) {
  if (e.shiftKey && lastClickedIndex.value >= 0) {
    const min = Math.min(lastClickedIndex.value, index)
    const max = Math.max(lastClickedIndex.value, index)
    const s = new Set(selectedIds.value)
    for (let i = min; i <= max; i++) s.add(gallery.value[i].id)
    selectedIds.value = s
  } else {
    const s = new Set(selectedIds.value)
    if (s.has(imageId)) s.delete(imageId)
    else s.add(imageId)
    selectedIds.value = s
    lastClickedIndex.value = index
  }
}

function selectAll() {
  selectedIds.value = new Set(gallery.value.map(img => img.id))
}

function onKeydown(e: KeyboardEvent) {
  if (!isManage.value) return
  if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
    e.preventDefault()
    selectAll()
  }
}

watch(isManage, (on) => {
  if (on) {
    document.addEventListener('keydown', onKeydown)
    orderSnapshot.value = (card.value?.images ?? []).map(img => ({ id: img.id, order: img.order }))
  } else {
    document.removeEventListener('keydown', onKeydown)
    selectedIds.value = new Set()
    lastClickedIndex.value = -1
    pendingDeleteId.value = null
    pendingBatchDelete.value = false
    orderSnapshot.value = null
  }
})

onUnmounted(() => document.removeEventListener('keydown', onKeydown))

// ── Manage mode — per-image controls ─────────────────────────────────────────
const pendingDeleteId = ref<number | null>(null)
const pendingBatchDelete = ref(false)

function doSetLead(imageId: number) {
  const c = ctx.value
  if (!c) return
  store.setLeadImage(c.cardId, imageId)
  ui.markCardDirty(c.cardId)
}

function doToggleIncluded(imageId: number) {
  const c = ctx.value
  if (!c) return
  store.toggleImageIncluded(c.cardId, imageId)
  ui.markCardDirty(c.cardId)
}

function doRemove(imageId: number) {
  const c = ctx.value
  if (!c) return
  store.removeImage(c.cardId, imageId)
  ui.markCardDirty(c.cardId)
  pendingDeleteId.value = null
  const s = new Set(selectedIds.value)
  s.delete(imageId)
  selectedIds.value = s
}

function clearSelection() {
  selectedIds.value = new Set()
  lastClickedIndex.value = -1
}

function deleteSelected() {
  const c = ctx.value
  if (!c || selectedIds.value.size === 0) return
  for (const id of selectedIds.value) store.removeImage(c.cardId, id)
  ui.markCardDirty(c.cardId)
  selectedIds.value = new Set()
  pendingDeleteId.value = null
  pendingBatchDelete.value = false
}

// ── Per-photo detail ─────────────────────────────────────────────────────────
const detailIdx = ref<number | null>(null)
const detailImage = computed(() => detailIdx.value != null ? gallery.value[detailIdx.value] : null)

function openDetail(i: number) { detailIdx.value = i }
function closeDetail() { detailIdx.value = null }
function prevDetail() { if (detailIdx.value != null && detailIdx.value > 0) detailIdx.value-- }
function nextDetail() { if (detailIdx.value != null && detailIdx.value < gallery.value.length - 1) detailIdx.value++ }

function onDetailAlt(imageId: number, alt: string) {
  const c = ctx.value
  if (c) { store.setImageMeta(c.cardId, imageId, { alt }); ui.markCardDirty(c.cardId) }
}
function onDetailCarId(imageId: number, carId: string | null) {
  const c = ctx.value
  if (c) { store.setImageMeta(c.cardId, imageId, { carId }); ui.markCardDirty(c.cardId) }
}

function onDetailLiveryId(imageId: number, liveryId: number | null) {
  const c = ctx.value
  if (c) { store.setImageMeta(c.cardId, imageId, { liveryId }); ui.markCardDirty(c.cardId) }
}

function onTriggerMultiCar(carId: string) {
  const c = ctx.value
  if (c) ui.triggerMultiCar(c.cardId, carId)
}

// carIds of all other gallery images excluding the current detail image.
const otherTaggedCarIds = computed(() => {
  const detailId = detailImage.value?.id
  return gallery.value
    .filter(img => img.id !== detailId && !!img.carId)
    .map(img => img.carId as string)
})

// ── Manage mode — drag reorder ───────────────────────────────────────────────
const orderSnapshot = ref<{ id: number; order: number }[] | null>(null)
const dragFromIdx = ref(-1)
const dropIdx = ref(-1)

const orderChanged = computed(() => {
  const snap = orderSnapshot.value
  if (!snap || !card.value) return false
  const snapMap = new Map(snap.map(s => [s.id, s.order]))
  return card.value.images.some(img => snapMap.get(img.id) !== img.order)
})

function undoReorder() {
  const c = ctx.value
  const snap = orderSnapshot.value
  if (!c || !snap) return
  store.restoreImageOrders(c.cardId, snap)
  ui.markCardDirty(c.cardId)
}

function onMgrDragStart(i: number) {
  dragFromIdx.value = i
  dropIdx.value = -1
}

function onMgrDragOver(i: number) {
  if (dragFromIdx.value >= 0) dropIdx.value = i
}

function onMgrDrop(i: number) {
  const c = ctx.value
  if (c && dragFromIdx.value >= 0 && dragFromIdx.value !== i) {
    store.reorderImages(c.cardId, dragFromIdx.value, i)
    ui.markCardDirty(c.cardId)
  }
  dragFromIdx.value = -1
  dropIdx.value = -1
}

function onMgrDragEnd() {
  dragFromIdx.value = -1
  dropIdx.value = -1
}

// ── Manage mode — upload ──────────────────────────────────────────────────────
const uploadProgress = ref<{ done: number; total: number } | null>(null)
const manageUploadRef = ref<HTMLInputElement | null>(null)

const SUPPORTED = new Set(['image/jpeg', 'image/jpg', 'image/png', 'image/webp'])

async function onManageUpload(e: Event) {
  const c = ctx.value
  const cv = card.value
  if (!c || !cv) return
  const all = Array.from((e.target as HTMLInputElement).files ?? [])
  if (manageUploadRef.value) manageUploadRef.value.value = ''
  const files = all.filter(f => SUPPORTED.has(f.type) || (!f.name.match(/\.(heic|heif|avif)$/i) && f.type.startsWith('image/')))
  if (!files.length) return
  const cardCtx = { name: cv.name, subtitle: cv.subtitle, collections: cv.collections, id: c.cardId }
  const startIndex = cv.images.length
  uploadProgress.value = { done: 0, total: files.length }
  for (let i = 0; i < files.length; i++) {
    try {
      const { id: dbId, path, thumbPath, stagePath } = await api.uploadImage(files[i], cardCtx, startIndex + i)
      store.addImageToPool(c.cardId, path, thumbPath, stagePath, true, dbId)
      ui.markCardDirty(c.cardId)
    } catch (err) {
      console.warn('[image-manager] upload failed:', err)
    }
    uploadProgress.value!.done++
  }
  uploadProgress.value = null
}
</script>

<template>
  <div class="image-picker float_imagepicker_backdrop" :class="{ open: !!ctx }" @click.self="modal.closeImagePicker()">
    <div class="image-picker-panel float_imagepicker_panel" :class="{ 'mgr-panel': isManage }">

      <!-- ── Pick mode ─────────────────────────────────────────────────────── -->
      <template v-if="!isManage">
        <div class="image-picker-head">
          <span>Choose a feature image</span>
          <button class="image-picker-close" aria-label="Close" @click="modal.closeImagePicker()">×</button>
        </div>
        <div class="image-picker-grid">
          <img v-for="img in gallery" :key="img.id" :src="img.path" @click="pick(img.path)" />
        </div>
        <label class="image-picker-upload">
          Or upload a new image…
          <input type="file" accept="image/*" @change="onPickUpload" />
        </label>
      </template>

      <!-- ── Manage mode ───────────────────────────────────────────────────── -->
      <template v-else>
        <div class="image-picker-head">
          <span>Card Photos</span>
          <button class="image-picker-close" aria-label="Close" @click="modal.closeImagePicker()">×</button>
        </div>
        <div class="mgr-grid" @dragleave.self="dropIdx = -1">
          <template v-for="(img, i) in gallery" :key="img.id">
          <div v-if="dragFromIdx >= 0 && dropIdx === i && dragFromIdx !== i" class="mgr-drop-indicator" />
          <div
            class="mgr-thumb"
            :class="{ excluded: img.included === false, selected: selectedIds.has(img.id), 'mgr-dragging': dragFromIdx === i }"
            draggable="true"
            @click="onThumbClick($event, img.id, i)"
            @dragstart.stop="onMgrDragStart(i)"
            @dragover.prevent="onMgrDragOver(i)"
            @drop.prevent="onMgrDrop(i)"
            @dragend="onMgrDragEnd"
          >
            <img :src="img.thumbPath ?? img.path" />
            <div class="mgr-controls" @click.stop>
              <div class="mgr-ctrl-left">
                <button
                  class="mgr-ctrl mgr-ctrl-lead"
                  :class="{ 'is-lead': img.order === 0 }"
                  title="Set as lead image"
                  @click="doSetLead(img.id)"
                >★</button>
                <button
                  class="mgr-ctrl mgr-ctrl-pool"
                  :class="{ included: img.included !== false }"
                  :title="img.included === false ? 'Add to slideshow' : 'Remove from slideshow'"
                  @click="doToggleIncluded(img.id)"
                >●</button>
              </div>
              <div class="mgr-ctrl-right">
                <button
                  class="mgr-ctrl mgr-ctrl-detail"
                  title="Photo details"
                  @click="openDetail(i)"
                >⤢</button>
                <button
                  class="mgr-ctrl mgr-ctrl-del"
                  title="Remove from card"
                  @click="pendingDeleteId = img.id"
                >✕</button>
              </div>
            </div>
            <div v-if="pendingDeleteId === img.id" class="mgr-delete-confirm" @click.stop>
              <span>Remove?</span>
              <button @click.stop="doRemove(img.id)">Yes</button>
              <button @click.stop="pendingDeleteId = null">No</button>
            </div>
          </div>
          </template>
        </div>

        <div class="mgr-footer">
          <template v-if="uploadProgress">
            <span class="mgr-progress">Uploading {{ uploadProgress.done }}/{{ uploadProgress.total }}…</span>
          </template>
          <template v-else-if="gallery.length === 0">
            <button class="mgr-action-btn mgr-add-btn" @click="modal.closeImagePicker()">Done</button>
          </template>
          <template v-else-if="pendingBatchDelete">
            <span class="mgr-confirm-label">Delete {{ selectedIds.size }} image{{ selectedIds.size !== 1 ? 's' : '' }}?</span>
            <div class="mgr-footer-right">
              <button class="mgr-action-btn mgr-delete-btn" @click="pendingBatchDelete = false">Cancel</button>
              <button class="mgr-action-btn mgr-add-btn" @click="deleteSelected">Yes, Delete</button>
            </div>
          </template>
          <template v-else>
            <div class="mgr-footer-left">
              <label class="mgr-action-btn mgr-add-btn">
                + Add Photos
                <input
                  ref="manageUploadRef"
                  type="file"
                  accept="image/jpeg,image/png,image/webp"
                  webkitdirectory
                  multiple
                  style="display:none"
                  @change="onManageUpload"
                />
              </label>
              <button v-if="orderChanged" class="mgr-action-btn mgr-cancel-btn" @click="undoReorder">Undo Order</button>
            </div>
            <div class="mgr-footer-right">
              <template v-if="selectedIds.size > 0">
                <button class="mgr-action-btn mgr-cancel-btn" @click="clearSelection">Select None</button>
                <button class="mgr-action-btn mgr-delete-btn" @click="pendingBatchDelete = true">Delete ({{ selectedIds.size }})</button>
              </template>
              <button class="mgr-action-btn mgr-add-btn" @click="modal.closeImagePicker()">Done</button>
            </div>
          </template>
        </div>
      </template>

    </div>
  </div>

  <!-- Per-photo detail shadowbox -->
  <PhotoDetail
    v-if="detailImage"
    :image="detailImage"
    :card-car-id="card?.carId"
    :card-id="ctx?.cardId"
    :other-tagged-car-ids="otherTaggedCarIds"
    :position="detailIdx! + 1"
    :total="gallery.length"
    :has-prev="detailIdx! > 0"
    :has-next="detailIdx! < gallery.length - 1"
    @close="closeDetail"
    @prev="prevDetail"
    @next="nextDetail"
    @update:alt="onDetailAlt"
    @update:car-id="onDetailCarId"
    @update:livery-id="onDetailLiveryId"
    @trigger-multi-car="onTriggerMultiCar"
  />
</template>

<style scoped>
.mgr-panel {
  max-width: 700px;
}

/* ── Manage grid ─────────────────────────────────────────────────────────── */
.mgr-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  overflow-y: auto;
  min-height: 0;
  padding-bottom: 2px;
}

.mgr-thumb {
  position: relative;
  width: 96px;
  height: 60px;
  border-radius: 2px;
  overflow: hidden;
  border: 2px solid transparent;
  background: var(--stage-bg);
  flex: 0 0 auto;
  cursor: pointer;
  transition: border-color 0.12s ease, opacity 0.12s ease;
}
.mgr-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.mgr-thumb.selected {
  border-color: var(--accent);
}
.mgr-thumb.excluded {
  opacity: 0.4;
}
.mgr-thumb.excluded img {
  filter: grayscale(60%);
}
.mgr-thumb.excluded.selected {
  opacity: 0.7;
}
.mgr-thumb.mgr-dragging {
  opacity: 0.3;
}
.mgr-drop-indicator {
  width: 3px;
  height: 60px;
  flex: 0 0 auto;
  background: var(--accent);
  border-radius: 2px;
  box-shadow: 0 0 8px rgba(201,162,39,0.7);
}

/* ── Controls ────────────────────────────────────────────────────────────── */
.mgr-controls {
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
.mgr-thumb:hover .mgr-controls {
  opacity: 1;
}
.mgr-ctrl-left {
  display: flex;
  gap: 2px;
}
.mgr-ctrl-right {
  display: flex;
  gap: 2px;
}
.mgr-ctrl-detail:hover { color: var(--accent, #c9aa71); }
.mgr-ctrl {
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
.mgr-ctrl-lead.is-lead,
.mgr-ctrl-lead:hover { color: var(--accent); }
.mgr-ctrl-pool.included { color: var(--accent); }
.mgr-ctrl-pool:not(.included) { color: rgba(255, 255, 255, 0.3); }
.mgr-ctrl-del:hover { background: rgba(170, 30, 30, 0.9); color: #fff; }

.mgr-thumb.excluded .mgr-ctrl { background: rgba(220,220,220,0.75); color: rgba(0,0,0,0.55); }
.mgr-thumb.excluded .mgr-ctrl-lead.is-lead,
.mgr-thumb.excluded .mgr-ctrl-lead:hover { color: #9a7800; }
.mgr-thumb.excluded .mgr-ctrl-pool.included { color: #9a7800; }
.mgr-thumb.excluded .mgr-ctrl-pool:not(.included) { color: rgba(0,0,0,0.3); }
.mgr-thumb.excluded .mgr-ctrl-del:hover { background: rgba(170,30,30,0.85); color: #fff; }

/* ── Per-image delete confirm ────────────────────────────────────────────── */
.mgr-delete-confirm {
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
  border-radius: 2px;
  z-index: 15;
}
.mgr-delete-confirm span {
  width: 100%;
  text-align: center;
  color: var(--accent);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.04em;
}
.mgr-delete-confirm button {
  border-radius: 3px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  padding: 3px 10px;
  cursor: pointer;
  transition: background .15s ease, border-color .15s ease, box-shadow .15s ease;
}
.mgr-delete-confirm button:first-of-type { background:#5c0000; color:#fff; border:1px solid #7a0000; }
.mgr-delete-confirm button:first-of-type:hover { background:#cc0000; border-color:#ff4444; box-shadow:0 0 10px rgba(200,0,0,0.8); }
.mgr-delete-confirm button:last-of-type { background:#7a5800; color:#fff; border:1px solid #a07800; }
.mgr-delete-confirm button:last-of-type:hover { background:#ffc200; border-color:#ffe870; box-shadow:0 0 10px rgba(255,194,0,0.85); }

/* ── Footer ──────────────────────────────────────────────────────────────── */
.mgr-footer {
  margin-top: 14px;
  padding-top: 12px;
  border-top: 1px solid var(--panel-edge);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.mgr-footer-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.mgr-footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
.mgr-action-btn {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: #fff;
  padding: 6px 14px;
  border-radius: 3px;
  cursor: pointer;
  transition: background .15s ease, border-color .15s ease, box-shadow .15s ease;
}
/* Red light — Delete */
.mgr-delete-btn { background:color-mix(in srgb, var(--danger) 40%, transparent); border:2px solid var(--danger); color:var(--danger-bright); }
.mgr-delete-btn:hover { background:color-mix(in srgb, var(--danger) 65%, transparent); border-color:var(--danger-bright); box-shadow:0 0 14px color-mix(in srgb, var(--danger-bright) 50%, transparent); }
/* Green light — Add Photos */
.mgr-add-btn { background:color-mix(in srgb, var(--success) 35%, transparent); border:2px solid var(--success); color:var(--success-bright); }
.mgr-add-btn:hover { background:var(--success-unlit); border-color:var(--success-bright); box-shadow:0 0 14px color-mix(in srgb, var(--success-bright) 50%, transparent); }
/* Yellow light — Cancel */
.mgr-cancel-btn { background:color-mix(in srgb, var(--accent) 30%, transparent); border:2px solid color-mix(in srgb, var(--accent) 60%, transparent); color:var(--accent); }
.mgr-cancel-btn:hover { background:color-mix(in srgb, var(--accent) 45%, transparent); border-color:var(--accent); box-shadow:0 0 14px color-mix(in srgb, var(--accent) 45%, transparent); }
.mgr-confirm-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--accent);
  letter-spacing: 0.04em;
  margin-right: 4px;
}

.mgr-progress {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--accent);
  letter-spacing: 0.04em;
}
</style>
