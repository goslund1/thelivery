<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useModalStore } from '../stores/modal'
import { useCardsStore } from '../stores/cards'
import type { CardImage } from '../types'

const modal = useModalStore()
const store = useCardsStore()

// ── Types ────────────────────────────────────────────────────────────────────

interface TextBox {
  id: string
  style: string
  content: string
  x: number   // fraction 0–1, left edge
  y: number   // fraction 0–1, top edge
  w: number   // fraction 0–1, width
  h: number   // fraction 0–1, height
  rotateDeg: number
  shearX: number
}

// ── State ────────────────────────────────────────────────────────────────────

const canvasRef    = ref<HTMLElement>()
const boxes        = ref<TextBox[]>([])
const selectedId   = ref<string | null>(null)
const previewSrc   = ref<string | null>(null)
const isLoading    = ref(false)
const photoId      = ref<number | null>(null)
const photos       = ref<CardImage[]>([])
const presetName   = ref('')
const newContent   = ref('')
const newStyle     = ref('POSTCARD')
const saving       = ref(false)
const saveMsg      = ref('')

const STYLES = ['POSTCARD', 'SIGNAL', 'GHOST']

const selected = computed(() => boxes.value.find(b => b.id === selectedId.value) ?? null)

// ── Init / teardown ──────────────────────────────────────────────────────────

watch(() => modal.ogMaker, (cfg) => {
  if (!cfg) return
  boxes.value      = cfg.boxes ? cfg.boxes.map(b => ({ ...b, id: (b as any).id ?? crypto.randomUUID() })) : []
  presetName.value = cfg.presetName ?? ''
  photoId.value    = cfg.photoId ?? null
  photos.value     = cfg.photos ?? []
  previewSrc.value = null
  selectedId.value = null
  if (photoId.value !== null) requestPreview()
}, { immediate: true })

onMounted(() => document.addEventListener('pointermove', onPointerMove))
onBeforeUnmount(() => document.removeEventListener('pointermove', onPointerMove))

// ── Drag state (non-reactive mutable) ───────────────────────────────────────

type DragKind = 'move' | 'tl' | 'tr' | 'bl' | 'br' | 'rotate'

interface DragState {
  boxId: string
  kind: DragKind
  startPx: number
  startPy: number
  startBox: TextBox
  startAngle: number  // for rotate only
}

let drag: DragState | null = null
let previewTimer: ReturnType<typeof setTimeout> | null = null

// ── Coordinate helpers ───────────────────────────────────────────────────────

function canvasRect() {
  return canvasRef.value?.getBoundingClientRect() ?? { width: 1, height: 1, left: 0, top: 0 }
}

function boxCenter(box: TextBox) {
  const r = canvasRect()
  return {
    cx: (box.x + box.w / 2) * r.width  + r.left,
    cy: (box.y + box.h / 2) * r.height + r.top,
  }
}

function clampBox(b: TextBox) {
  // Centre must stay on canvas so at least one handle is reachable.
  const cx = Math.max(0.02, Math.min(0.98, b.x + b.w / 2))
  const cy = Math.max(0.02, Math.min(0.98, b.y + b.h / 2))
  b.x = cx - b.w / 2
  b.y = cy - b.h / 2
  b.w = Math.max(0.04, b.w)
  b.h = Math.max(0.02, b.h)
}

// ── Drag entry points ────────────────────────────────────────────────────────

function startMove(e: PointerEvent, id: string) {
  e.preventDefault()
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  const box = boxes.value.find(b => b.id === id)!
  selectedId.value = id
  drag = { boxId: id, kind: 'move', startPx: e.clientX, startPy: e.clientY, startBox: { ...box }, startAngle: 0 }
}

function startResize(e: PointerEvent, id: string, corner: 'tl' | 'tr' | 'bl' | 'br') {
  e.preventDefault()
  e.stopPropagation()
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  const box = boxes.value.find(b => b.id === id)!
  drag = { boxId: id, kind: corner, startPx: e.clientX, startPy: e.clientY, startBox: { ...box }, startAngle: 0 }
}

function startRotate(e: PointerEvent, id: string) {
  e.preventDefault()
  e.stopPropagation()
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  const box = boxes.value.find(b => b.id === id)!
  const { cx, cy } = boxCenter(box)
  const startAngle = Math.atan2(e.clientY - cy, e.clientX - cx) * 180 / Math.PI
  drag = { boxId: id, kind: 'rotate', startPx: e.clientX, startPy: e.clientY, startBox: { ...box }, startAngle }
}

// ── Pointer move / up ────────────────────────────────────────────────────────

function onPointerMove(e: PointerEvent) {
  if (!drag) return
  const r = canvasRect()
  const dpx = e.clientX - drag.startPx
  const dpy = e.clientY - drag.startPy
  const dfx = dpx / r.width
  const dfy = dpy / r.height
  const sb  = drag.startBox
  const box = boxes.value.find(b => b.id === drag!.boxId)
  if (!box) return

  switch (drag.kind) {
    case 'move':
      box.x = sb.x + dfx
      box.y = sb.y + dfy
      clampBox(box)
      break
    case 'br':
      box.w = Math.max(0.04, sb.w + dfx)
      box.h = Math.max(0.02, sb.h + dfy)
      break
    case 'bl':
      box.x = sb.x + dfx
      box.w = Math.max(0.04, sb.w - dfx)
      box.h = Math.max(0.02, sb.h + dfy)
      break
    case 'tr':
      box.y = sb.y + dfy
      box.w = Math.max(0.04, sb.w + dfx)
      box.h = Math.max(0.02, sb.h - dfy)
      break
    case 'tl':
      box.x = sb.x + dfx
      box.y = sb.y + dfy
      box.w = Math.max(0.04, sb.w - dfx)
      box.h = Math.max(0.02, sb.h - dfy)
      break
    case 'rotate': {
      const { cx, cy } = boxCenter(box)
      const currentAngle = Math.atan2(e.clientY - cy, e.clientX - cx) * 180 / Math.PI
      box.rotateDeg = sb.rotateDeg + (currentAngle - drag.startAngle)
      break
    }
  }
}

function onPointerUp() {
  if (!drag) return
  drag = null
  schedulePreview()
}

// ── Preview ──────────────────────────────────────────────────────────────────

function schedulePreview() {
  if (previewTimer) clearTimeout(previewTimer)
  previewTimer = setTimeout(requestPreview, 200)
}

async function requestPreview() {
  if (photoId.value === null) return
  const config = buildConfig()
  isLoading.value = true
  try {
    const token = localStorage.getItem('auth_token') ?? ''
    const res = await fetch('/share/preview', {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
      body: JSON.stringify(config),
    })
    if (res.ok) {
      const blob = await res.blob()
      if (previewSrc.value) URL.revokeObjectURL(previewSrc.value)
      previewSrc.value = URL.createObjectURL(blob)
    }
  } finally {
    isLoading.value = false
  }
}

function buildConfig() {
  return {
    photoId: photoId.value ?? 0,
    logoVisible: false,
    textBoxes: boxes.value.map(({ id: _id, ...b }) => b),
  }
}

// ── Box operations ───────────────────────────────────────────────────────────

function addBox() {
  if (!newContent.value.trim()) return
  const id = crypto.randomUUID()
  boxes.value.push({
    id,
    style: newStyle.value,
    content: newContent.value.trim().toUpperCase(),
    x: 0.05, y: 0.72, w: 0.5, h: 0.16,
    rotateDeg: 0, shearX: 0,
  })
  selectedId.value = id
  newContent.value = ''
  schedulePreview()
}

function deleteSelected() {
  if (!selectedId.value) return
  boxes.value = boxes.value.filter(b => b.id !== selectedId.value)
  selectedId.value = null
  schedulePreview()
}

function setPhoto(img: CardImage) {
  photoId.value = img.id
  schedulePreview()
}

// Watchers for toolbar controls — fire preview on settle.
watch(() => selected.value?.rotateDeg, () => schedulePreview())
watch(() => selected.value?.shearX,    () => schedulePreview())
watch(() => selected.value?.style,     () => schedulePreview())

// ── Save preset ──────────────────────────────────────────────────────────────

async function savePreset() {
  if (!presetName.value.trim()) { saveMsg.value = 'Name required'; return }
  saving.value = true
  saveMsg.value = ''
  try {
    const token = localStorage.getItem('auth_token') ?? ''
    const res = await fetch('/api/og-presets', {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
      body: JSON.stringify({ name: presetName.value.trim(), config: buildConfig() }),
    })
    saveMsg.value = res.ok ? 'Saved!' : 'Save failed'
    setTimeout(() => { saveMsg.value = '' }, 2000)
  } finally {
    saving.value = false
  }
}

async function saveToCard() {
  const cardId = modal.ogMaker?.cardId
  if (!cardId) return
  const card = store.byId(cardId)
  if (!card) return
  card.shareOverlayConfig = buildConfig()
  await store.save(cardId)
  saveMsg.value = 'Saved to card!'
  setTimeout(() => { saveMsg.value = ''; modal.closeOgMaker() }, 1200)
}

// ── CSS helpers ──────────────────────────────────────────────────────────────

function boxStyle(box: TextBox) {
  const skewDeg = Math.atan(box.shearX) * 180 / Math.PI
  return {
    left:      `${box.x * 100}%`,
    top:       `${box.y * 100}%`,
    width:     `${box.w * 100}%`,
    height:    `${box.h * 100}%`,
    transform: `rotate(${box.rotateDeg}deg) skewX(${skewDeg}deg)`,
  }
}

function rotHandleStyle(box: TextBox) {
  return {
    left:      `${(box.x + box.w / 2) * 100}%`,
    top:       `${box.y * 100}%`,
    transform: `translate(-50%, -200%) rotate(${box.rotateDeg}deg)`,
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modal.ogMaker" class="ogm-backdrop" @click.self="modal.closeOgMaker()">
      <div class="ogm-modal" @pointerup="onPointerUp">

        <!-- Header -->
        <div class="ogm-header">
          <input v-model="presetName" class="ogm-preset-name" placeholder="Preset name…" />
          <button class="ogm-btn ogm-btn--save" :disabled="saving" @click="savePreset">
            {{ saving ? 'Saving…' : 'Save Preset' }}
          </button>
          <button v-if="modal.ogMaker?.cardId" class="ogm-btn ogm-btn--card" :disabled="saving" @click="saveToCard">
            Save to Card
          </button>
          <span v-if="saveMsg" class="ogm-save-msg">{{ saveMsg }}</span>
          <button class="ogm-btn ogm-btn--close" @click="modal.closeOgMaker()">✕</button>
        </div>

        <!-- Canvas -->
        <div class="ogm-canvas-wrap" ref="canvasRef" @click.self="selectedId = null">
          <!-- Compositor PNG -->
          <img v-if="previewSrc" :src="previewSrc" class="ogm-preview" draggable="false" />
          <div v-else class="ogm-preview-empty">{{ photoId ? 'Loading…' : 'Choose a photo below' }}</div>

          <!-- Loading shimmer -->
          <div v-if="isLoading" class="ogm-loading" />

          <!-- Text box hit areas -->
          <div
            v-for="box in boxes"
            :key="box.id"
            class="ogm-box"
            :class="{ 'ogm-box--selected': selectedId === box.id }"
            :style="boxStyle(box)"
            @pointerdown="startMove($event, box.id)"
          >
            <!-- Corner handles -->
            <template v-if="selectedId === box.id">
              <div class="ogm-handle ogm-handle--tl" @pointerdown="startResize($event, box.id, 'tl')" />
              <div class="ogm-handle ogm-handle--tr" @pointerdown="startResize($event, box.id, 'tr')" />
              <div class="ogm-handle ogm-handle--bl" @pointerdown="startResize($event, box.id, 'bl')" />
              <div class="ogm-handle ogm-handle--br" @pointerdown="startResize($event, box.id, 'br')" />
            </template>
          </div>

          <!-- Rotation handle (floats above selected box) -->
          <div
            v-if="selectedId && selected"
            class="ogm-rot-handle"
            :style="rotHandleStyle(selected)"
            @pointerdown="startRotate($event, selectedId)"
          />
        </div>

        <!-- Toolbar (selected box) -->
        <div v-if="selected" class="ogm-toolbar">
          <select v-model="selected.style" class="ogm-select">
            <option v-for="s in STYLES" :key="s" :value="s">{{ s }}</option>
          </select>
          <input v-model="selected.content" class="ogm-text-input" @change="schedulePreview()" />
          <label class="ogm-slider-label">
            Rotate
            <input type="range" v-model.number="selected.rotateDeg" min="-45" max="45" step="0.5" class="ogm-slider" />
            <span class="ogm-slider-val">{{ selected.rotateDeg.toFixed(1) }}°</span>
          </label>
          <label class="ogm-slider-label">
            Shear
            <input type="range" v-model.number="selected.shearX" min="-0.6" max="0.6" step="0.01" class="ogm-slider" />
            <span class="ogm-slider-val">{{ selected.shearX.toFixed(2) }}</span>
          </label>
          <button class="ogm-btn ogm-btn--delete" @click="deleteSelected">Delete</button>
        </div>

        <!-- Add box row -->
        <div class="ogm-add-row">
          <input
            v-model="newContent"
            class="ogm-text-input ogm-text-input--new"
            placeholder="Type stamp text…"
            @keydown.enter="addBox"
          />
          <select v-model="newStyle" class="ogm-select">
            <option v-for="s in STYLES" :key="s" :value="s">{{ s }}</option>
          </select>
          <button class="ogm-btn ogm-btn--add" @click="addBox">+ Add</button>
        </div>

        <!-- Photo strip -->
        <div v-if="photos.length" class="ogm-photo-strip">
          <img
            v-for="img in photos"
            :key="img.id"
            :src="img.thumbPath ?? img.path"
            class="ogm-photo-thumb"
            :class="{ 'ogm-photo-thumb--active': img.id === photoId }"
            @click="setPhoto(img)"
          />
        </div>

      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ogm-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1300;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.ogm-modal {
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 6px;
  width: min(960px, 100%);
  max-height: 90vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0;
}

/* Header */
.ogm-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--panel-edge);
}
.ogm-preset-name {
  flex: 1;
  background: var(--input-bg, rgba(255,255,255,0.07));
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--text);
  padding: 4px 8px;
  font-size: 0.85rem;
}
.ogm-save-msg { font-size: 0.8rem; color: var(--accent); }

/* Canvas */
.ogm-canvas-wrap {
  position: relative;
  width: 100%;
  aspect-ratio: 1200 / 630;
  background: #111;
  overflow: hidden;
  cursor: default;
}
.ogm-preview {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  pointer-events: none;
  user-select: none;
}
.ogm-preview-empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted, rgba(255,255,255,0.3));
  font-size: 0.9rem;
}
.ogm-loading {
  position: absolute;
  inset: 0;
  background: rgba(0,0,0,0.25);
  pointer-events: none;
  animation: ogm-pulse 0.8s ease-in-out infinite alternate;
}
@keyframes ogm-pulse { from { opacity: 0 } to { opacity: 1 } }

/* Text box hit areas */
.ogm-box {
  position: absolute;
  cursor: move;
  border: 1px solid transparent;
  box-sizing: border-box;
  transform-origin: center center;
}
.ogm-box--selected {
  border-color: var(--accent, #e8c84a);
}

/* Corner handles */
.ogm-handle {
  position: absolute;
  width: 10px;
  height: 10px;
  background: var(--accent, #e8c84a);
  border: 1px solid var(--panel);
  border-radius: 2px;
}
.ogm-handle--tl { top: -5px;  left: -5px;  cursor: nw-resize; }
.ogm-handle--tr { top: -5px;  right: -5px; cursor: ne-resize; }
.ogm-handle--bl { bottom: -5px; left: -5px;  cursor: sw-resize; }
.ogm-handle--br { bottom: -5px; right: -5px; cursor: se-resize; }

/* Rotation handle */
.ogm-rot-handle {
  position: absolute;
  width: 12px;
  height: 12px;
  background: var(--highlight, #e8748a);
  border: 1px solid var(--panel);
  border-radius: 50%;
  cursor: crosshair;
  transform-origin: center center;
}

/* Toolbar */
.ogm-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid var(--panel-edge);
  border-bottom: 1px solid var(--panel-edge);
}
.ogm-slider-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.78rem;
  color: var(--muted, rgba(255,255,255,0.5));
}
.ogm-slider { width: 90px; accent-color: var(--accent, #e8c84a); }
.ogm-slider-val { font-size: 0.75rem; color: var(--text); min-width: 36px; }

/* Add row */
.ogm-add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
}
.ogm-text-input {
  background: var(--input-bg, rgba(255,255,255,0.07));
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--text);
  padding: 4px 8px;
  font-size: 0.85rem;
}
.ogm-text-input--new { flex: 1; }
.ogm-select {
  background: var(--input-bg, rgba(255,255,255,0.07));
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--text);
  padding: 4px 6px;
  font-size: 0.8rem;
}

/* Photo strip */
.ogm-photo-strip {
  display: flex;
  gap: 6px;
  padding: 8px 12px;
  overflow-x: auto;
  border-top: 1px solid var(--panel-edge);
}
.ogm-photo-thumb {
  width: 80px;
  height: 45px;
  object-fit: cover;
  border-radius: 3px;
  cursor: pointer;
  border: 2px solid transparent;
  flex-shrink: 0;
  opacity: 0.7;
  transition: opacity 0.15s, border-color 0.15s;
}
.ogm-photo-thumb:hover { opacity: 1; }
.ogm-photo-thumb--active {
  border-color: var(--accent, #e8c84a);
  opacity: 1;
}

/* Buttons */
.ogm-btn {
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: transparent;
  color: var(--text);
  font-size: 0.8rem;
  cursor: pointer;
}
.ogm-btn:hover { background: var(--panel-edge); }
.ogm-btn--save  { border-color: var(--accent, #e8c84a); color: var(--accent, #e8c84a); }
.ogm-btn--card  { border-color: var(--highlight, #e8748a); color: var(--highlight, #e8748a); }
.ogm-btn--add  { border-color: var(--accent, #e8c84a); color: var(--accent, #e8c84a); }
.ogm-btn--delete { border-color: var(--highlight, #e8748a); color: var(--highlight, #e8748a); }
.ogm-btn--close { margin-left: auto; font-size: 1rem; }
</style>
