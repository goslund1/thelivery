<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { useLiveriesStore } from '../stores/liveries'
import { useAuthStore } from '../stores/auth'
import { useCarsStore } from '../stores/cars'
import { useToastsStore } from '../stores/toasts'
import { api } from '../api'
import { useScrollLock } from '../composables/useScrollLock'
import CarPicker from './CarPicker.vue'
import DrawerPanel from './DrawerPanel.vue'
import { useAssessFailures, type FailedAssess } from '../composables/useAssessFailures'
import type { Card } from '../types'

const store = useCardsStore()
const modal = useModalStore()
const { lockScroll, unlockScroll } = useScrollLock()
const liveriesStore = useLiveriesStore()
const auth = useAuthStore()
const carsStore = useCarsStore()
const toasts = useToastsStore()
carsStore.load()

const cardsWithImages = computed(() =>
  store.cards.filter(c => !c.isLegend && c.images.length > 0)
)

const currentIndex = ref(0)
const currentCard = computed<Card | null>(() => cardsWithImages.value[currentIndex.value] ?? null)
const done = computed(() => currentIndex.value >= cardsWithImages.value.length)

// Per-image selection
const selectedIds = ref<Set<number>>(new Set())

// Derived from actual data — persists across modal open/close
const assignedIds = computed(() =>
  new Set((currentCard.value?.images ?? []).filter(i => i.liveryId != null).map(i => i.id))
)

const allAssigned = computed(() => {
  const imgs = currentCard.value?.images ?? []
  return imgs.length > 0 && imgs.every(i => assignedIds.value.has(i.id))
})

const allSelected = computed(() => {
  const imgs = currentCard.value?.images ?? []
  return imgs.length > 0 && imgs.every(i => selectedIds.value.has(i.id))
})

function toggleImage(id: number) {
  const s = new Set(selectedIds.value)
  if (s.has(id)) s.delete(id); else s.add(id)
  selectedIds.value = s
}
function toggleAll() {
  if (allSelected.value) {
    selectedIds.value = new Set()
  } else {
    selectedIds.value = new Set((currentCard.value?.images ?? []).map(i => i.id))
  }
}

// Per-batch assignment state
const carId = ref<string | null>(null)
const liveryName = ref('')
const showCarRequired = ref(false)
const liveryNameValid = computed(() => liveryName.value.trim().length > 0)
const canAssign = computed(() =>
  selectedIds.value.size > 0 && liveryNameValid.value && !batchProcessing.value
)

// Live filename preview — shown in header, XXX for series number
const filenamePreview = computed(() => {
  const car = carId.value ? carsStore.byId(carId.value) : null
  const game = car?.game?.toUpperCase() ?? 'FHX'

  function slug(s: string) {
    return s.replace(/[^a-zA-Z0-9]+/g, '_').replace(/^_|_$/g, '')
  }

  const parts: string[] = [game]
  if (car) {
    parts.push(slug(car.make))
    parts.push(slug(car.model))
    if (car.year) parts.push(String(car.year))
  }
  const lv = liveryName.value.trim()
  if (lv) parts.push(slug(lv))
  const today = new Date()
  const dateStr = today.getFullYear().toString()
    + String(today.getMonth() + 1).padStart(2, '0')
    + String(today.getDate()).padStart(2, '0')
  parts.push('XXX')
  parts.push(dateStr)
  parts.push('xxxxxx')
  parts.push('WxH')
  return parts.filter(Boolean).join('_') + '.jpg'
})

const batchProcessing = ref(false)
const toastDrawerOpen = ref(false)

const { failedAssess, add: addFailure, remove: removeFailure } = useAssessFailures()

const retryingId = ref<number | null>(null)

async function retryAssess(f: FailedAssess) {
  retryingId.value = f.liveryId
  toastDrawerOpen.value = true
  const toastId = toasts.push(`Retry: ${f.cardName}`, [
    { text: `Assessing ${f.liveryName}…`, status: 'processing' },
  ])
  const itemId = toasts.toasts.find(t => t.id === toastId)!.items[0].id
  try {
    const r = await api.assessLiveryColor(f.liveryId)
    const colors = r.primary + (r.secondary ? ' / ' + r.secondary : '')
    toasts.updateItem(toastId, itemId, { status: 'done', text: 'Colors assessed', detail: colors })
    removeFailure(f.liveryId)
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e)
    const isQuota = msg.includes('429') || msg.toLowerCase().includes('quota') || msg.toLowerCase().includes('credit')
    toasts.updateItem(toastId, itemId, {
      status: 'error',
      text: isQuota ? 'AI quota exceeded' : 'Color assess failed',
      detail: isQuota ? 'retry later' : msg,
    })
  } finally {
    retryingId.value = null
  }
}

async function retryAll() {
  for (const f of [...failedAssess.value]) {
    await retryAssess(f)
  }
}

// Auto-close drawer once all toasts have faded and been removed
watch(() => toasts.toasts.length, (len) => {
  if (len === 0) toastDrawerOpen.value = false
})

// Reset per-card state when card changes
watch(currentCard, (card) => {
  selectedIds.value = new Set()
  batchProcessing.value = false
  carId.value = card?.carId ?? null
  liveryName.value = card?.name?.trim() || ''
}, { immediate: true })

watch(() => modal.imageMigrationOpen, (open) => {
  if (open) currentIndex.value = 0
})

function onKeydown(e: KeyboardEvent) {
  if (!modal.imageMigrationOpen) return
  if (e.key === 'Enter' && allAssigned.value && !batchProcessing.value) {
    e.preventDefault()
    nextCard()
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('keydown', onKeydown)
}
onUnmounted(() => window.removeEventListener('keydown', onKeydown))

async function assignSelected() {
  const card = currentCard.value
  if (!card || !auth.isAuthenticated || !canAssign.value) return

  if (!carId.value) {
    showCarRequired.value = true
    return
  }

  const ids = [...selectedIds.value]
  const name = liveryName.value.trim()
  const car = carsStore.byId(carId.value)
  const carLabel = car ? `${car.make} ${car.model}` : carId.value

  // Snapshot old paths before re-filing so we can patch figurePath on sections.
  const oldPaths = new Map<number, string>(
    (card.images ?? []).filter(i => ids.includes(i.id)).map(i => [i.id, i.path])
  )

  toastDrawerOpen.value = true
  const toastId = toasts.push(`${card.name} — ${carLabel}`, [
    { text: `${ids.length} image${ids.length !== 1 ? 's' : ''} — re-filing…`, status: 'pending' },
    { text: 'Creating livery…', status: 'pending' },
    { text: 'Assessing colors…', status: 'pending' },
  ])

  const [imgItemId, liveryItemId, assessItemId] = [0, 1, 2].map(i =>
    toasts.toasts.find(t => t.id === toastId)!.items[i].id
  )

  batchProcessing.value = true

  try {
    toasts.updateItem(toastId, liveryItemId, { status: 'processing', text: 'Creating livery…' })
    const livery = await liveriesStore.create({ carId: carId.value!, name })
    toasts.updateItem(toastId, liveryItemId, { status: 'done', text: `Livery: ${name}`, detail: livery.serial })

    toasts.updateItem(toastId, imgItemId, { status: 'processing' })
    const result = await api.migrateImages(ids, carId.value!, livery.id)

    for (const updated of result.migrated) {
      store.setImageMeta(card.id, updated.id, {
        carId: updated.carId,
        liveryId: updated.liveryId,
        path: updated.path,
        thumbPath: updated.thumbPath,
        stagePath: updated.stagePath,
      })
      // If any text section's figurePath pointed at this image's old path, update it.
      const oldPath = oldPaths.get(updated.id)
      if (oldPath) {
        for (const section of card.sections) {
          if (section.type === 'text' && section.figurePath === oldPath) {
            store.setFigure(card.id, section.key, updated.path)
          }
        }
      }
    }
    await store.save(card.id)

    toasts.updateItem(toastId, imgItemId, { status: 'done', text: `${result.migrated.length} image${result.migrated.length !== 1 ? 's' : ''} migrated` })

    selectedIds.value = new Set()
    carId.value = null
    liveryName.value = currentCard.value?.name?.trim() || ''

    if (result.migrated.length === 0) {
      toasts.updateItem(toastId, assessItemId, { status: 'done', text: 'Assess skipped', detail: 'no new files' })
      return
    }
    toasts.updateItem(toastId, assessItemId, { status: 'processing', text: 'Assessing colors…' })
    api.assessLiveryColor(livery.id)
      .then(r => {
        const colors = r.primary + (r.secondary ? ' / ' + r.secondary : '')
        toasts.updateItem(toastId, assessItemId, { status: 'done', text: 'Colors assessed', detail: colors })
      })
      .catch((e: unknown) => {
        const msg = e instanceof Error ? e.message : String(e)
        toasts.updateItem(toastId, assessItemId, {
          status: 'done',
          text: 'Queued for retry',
          detail: msg.slice(0, 40),
        })
        addFailure({ liveryId: livery.id, liveryName: name, cardName: card.name })
      })
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    const t = toasts.toasts.find(t => t.id === toastId)
    if (t) {
      for (const item of t.items) {
        if (item.status === 'processing') {
          toasts.updateItem(toastId, item.id, { status: 'error', text: msg })
        } else if (item.status === 'pending') {
          toasts.updateItem(toastId, item.id, { status: 'error', text: 'Skipped' })
        }
      }
    }
    console.error('[ImageMigration] assignSelected failed:', e)
  } finally {
    batchProcessing.value = false
  }
}

function nextCard() { currentIndex.value++ }
function prevCard() { if (currentIndex.value > 0) currentIndex.value-- }

function close() { modal.closeImageMigration() }
</script>

<template>
  <Teleport to="body">
    <div v-if="modal.imageMigrationOpen" class="imm-overlay" @click.self="close">
      <div class="imm-shell" @mouseenter="lockScroll" @mouseleave="unlockScroll">

        <!-- Main content column -->
        <div class="imm-main">
          <button class="imm-close" @click="close">×</button>

          <div v-if="done" class="imm-done">
            <p class="imm-done-msg">All {{ cardsWithImages.length }} cards visited.</p>

            <!-- Failed assess list -->
            <div v-if="failedAssess.length" class="imm-retry-panel">
              <div class="imm-retry-header">
                <span class="imm-retry-title">Color assess failed ({{ failedAssess.length }})</span>
                <button class="imm-btn-primary" :disabled="retryingId !== null" @click="retryAll">
                  Retry all
                </button>
              </div>
              <div class="imm-retry-list">
                <div v-for="f in failedAssess" :key="f.liveryId" class="imm-retry-row">
                  <span class="imm-retry-card">{{ f.cardName }}</span>
                  <span class="imm-retry-name">{{ f.liveryName }}</span>
                  <button
                    class="imm-btn-skip"
                    :disabled="retryingId === f.liveryId"
                    @click="retryAssess(f)"
                  >{{ retryingId === f.liveryId ? '…' : 'Retry' }}</button>
                </div>
              </div>
            </div>
            <p v-else class="imm-done-all-clear">All color assessments succeeded.</p>

            <button class="imm-btn-skip" @click="close">Close</button>
          </div>

          <template v-else-if="currentCard">
            <!-- Header -->
            <div class="imm-header">
              <div class="imm-header-top">
                <span class="imm-counter">{{ currentIndex + 1 }} / {{ cardsWithImages.length }}</span>
                <h2 class="imm-title">{{ currentCard.name }}</h2>
                <span v-if="failedAssess.length" class="imm-retry-badge" :title="`${failedAssess.length} assess failure${failedAssess.length !== 1 ? 's' : ''} queued`">{{ failedAssess.length }} retry</span>
              </div>
              <div class="imm-filename-preview">{{ filenamePreview }}</div>
            </div>

            <!-- Selection controls -->
            <div class="imm-select-bar">
              <button class="imm-select-all" @click="toggleAll">
                {{ allSelected ? 'Deselect all' : 'Select all' }}
              </button>
              <span class="imm-sel-count">
                {{ selectedIds.size }} selected
                <template v-if="assignedIds.size"> · {{ assignedIds.size }} assigned</template>
              </span>
            </div>

            <!-- Image grid -->
            <div class="imm-img-grid" :class="{ 'imm-img-grid--done': allAssigned }">
              <div v-if="allAssigned" class="imm-done-overlay">
                <span>Images</span>
                <span>Migrated</span>
              </div>
              <button
                v-for="img in currentCard.images.slice().sort((a,b) => a.order - b.order)"
                :key="img.id"
                class="imm-img-cell"
                :class="{
                  'imm-img-cell--selected': selectedIds.has(img.id),
                  'imm-img-cell--assigned': assignedIds.has(img.id) && !selectedIds.has(img.id),
                }"
                @click="toggleImage(img.id)"
              >
                <img
                  :src="img.thumbPath ?? img.path"
                  class="imm-thumb"
                  :alt="img.alt ?? ''"
                />
                <div class="imm-img-check">
                  <span v-if="selectedIds.has(img.id)">✓</span>
                  <span v-else-if="assignedIds.has(img.id)" class="imm-img-check--done">·</span>
                </div>
              </button>
            </div>

            <!-- Assignment controls -->
            <div class="imm-assign">
              <div v-if="showCarRequired" class="imm-car-gate">
                <span class="imm-car-gate-msg">A car must be selected before assigning images.</span>
                <button class="imm-car-gate-dismiss" @click="showCarRequired = false">Got it</button>
              </div>
              <div class="imm-row" :class="{ 'imm-row--highlight': showCarRequired }">
                <span class="imm-label">Car</span>
                <CarPicker :car-id="carId" @update:car-id="id => { carId = id; showCarRequired = false }" />
              </div>
              <div class="imm-row">
                <span class="imm-label">Livery</span>
                <input
                  class="imm-livery-input"
                  v-model="liveryName"
                  placeholder="Livery name…"
                />
              </div>
            </div>

            <!-- Actions -->
            <div class="imm-actions">
              <div class="imm-nav-btns">
                <button
                  class="imm-btn-skip"
                  :disabled="currentIndex === 0"
                  @click="prevCard"
                >← Prev</button>
                <button
                  class="imm-btn-skip"
                  :class="{ 'imm-btn-skip--ready': allAssigned }"
                  @click="nextCard"
                >Next →</button>
              </div>
              <button
                class="imm-btn-primary"
                :disabled="!canAssign"
                @click="assignSelected"
              >Assign selected →</button>
            </div>
          </template>
        </div>

        <!-- Migration Log drawer — right side, tab on left edge -->
        <DrawerPanel v-model:open="toastDrawerOpen" :width="200" :tab-width="16" side="right"
          background="var(--picker-glass-bg)">
          <template #header>Migration Log</template>
          <div class="imm-toast-scroll">
            <template v-if="toasts.toasts.length">
              <div
                v-for="toast in toasts.toasts"
                :key="toast.id"
                class="imm-toast-panel"
                :class="{ 'imm-toast-panel--fading': toast.fadingOut }"
              >
                <div class="imm-toast-title-row">
                  <span class="imm-toast-title">{{ toast.title }}</span>
                  <button class="imm-toast-dismiss" @click="toasts.dismiss(toast.id)">×</button>
                </div>
                <div
                  v-for="item in toast.items"
                  :key="item.id"
                  class="imm-toast-item"
                  :class="'imm-toast-item--' + item.status"
                >
                  <span class="imm-toast-dot" />
                  <span class="imm-toast-text">{{ item.text }}</span>
                  <span v-if="item.detail" class="imm-toast-detail">{{ item.detail }}</span>
                </div>
              </div>
            </template>
            <p v-else class="imm-toast-empty">No activity yet</p>
          </div>
        </DrawerPanel>

      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.imm-overlay {
  position: fixed;
  inset: 0;
  z-index: 600;
  background: rgba(0,0,0,0.78);
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Outer shell: no background — lets the overlay bleed through glass surfaces */
.imm-shell {
  position: relative;
  display: flex;
  flex-direction: row;
  align-items: stretch;
  max-height: 90vh;
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  overflow: hidden;
}

/* Main content column — primary glass surface */
.imm-main {
  position: relative;
  width: min(92vw, 560px);
  display: flex;
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  flex-direction: column;
  overflow-y: auto;
  flex-shrink: 0;
}

.imm-close {
  position: absolute;
  top: 8px; right: 10px;
  background: none; border: none;
  color: var(--muted); font-size: 18px;
  cursor: pointer; z-index: 1;
}
.imm-close:hover { color: var(--fg); }

/* Toast content inside DrawerPanel body slot */
.imm-toast-scroll {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.imm-toast-empty {
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--muted);
  margin: 0;
  opacity: 0.5;
}

/* Toast panels */
.imm-toast-panel {
  margin-bottom: 10px;
  transition: opacity .5s ease;
}
.imm-toast-panel--fading { opacity: 0; pointer-events: none; }

.imm-toast-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px 3px;
  border-bottom: 1px solid color-mix(in srgb, var(--panel-edge) 50%, transparent);
}
.imm-toast-title {
  font: 700 9px/1 'Oswald', sans-serif;
  letter-spacing: .06em;
  text-transform: uppercase;
  color: var(--fg);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.imm-toast-dismiss {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
  line-height: 1;
}
.imm-toast-dismiss:hover { color: var(--fg); }

.imm-toast-item {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 4px 6px;
  padding: 2px 8px;
  font: 9px/1.5 'JetBrains Mono', monospace;
}
.imm-toast-dot {
  width: 5px; height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--muted);
}
.imm-toast-item--processing .imm-toast-dot {
  background: var(--accent);
  animation: imm-pulse 1s ease-in-out infinite;
}
.imm-toast-item--done .imm-toast-dot { background: #4a9; }
.imm-toast-item--error .imm-toast-dot { background: #c44; }
.imm-toast-text { color: var(--fg); flex: 1; min-width: 0; overflow-wrap: break-word; overflow: hidden; }
.imm-toast-item--done .imm-toast-text { color: var(--muted); }
.imm-toast-item--error .imm-toast-text { color: #e07070; }
.imm-toast-detail { color: var(--muted); font-size: 8px; flex-shrink: 0; }
.imm-toast-item--done .imm-toast-detail { color: #4a9; }

@keyframes imm-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: .3; }
}

.imm-header {
  padding: 12px 16px 10px;
  border-bottom: 1px solid var(--panel-edge);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.imm-header-top {
  display: flex;
  align-items: baseline;
  gap: 12px;
}
.imm-counter {
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--muted);
  flex-shrink: 0;
}
.imm-title {
  font: 600 14px/1.2 'Oswald', sans-serif;
  color: var(--fg);
  margin: 0;
  text-transform: uppercase;
  letter-spacing: .05em;
}
.imm-retry-badge {
  margin-left: auto;
  font: 700 9px/1 'JetBrains Mono', monospace;
  padding: 2px 6px;
  border-radius: 3px;
  background: color-mix(in srgb, #c44 18%, transparent);
  border: 1px solid color-mix(in srgb, #c44 40%, transparent);
  color: #e07070;
  white-space: nowrap;
  cursor: default;
}

.imm-filename-preview {
  font: 9px/1.3 'JetBrains Mono', monospace;
  color: var(--muted);
  word-break: break-all;
  padding-left: 2px;
  opacity: 0.7;
  transition: color .15s;
}
.imm-filename-preview:not(:empty) { color: var(--accent); opacity: 1; }

.imm-select-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px;
  border-bottom: 1px solid var(--panel-edge);
}
.imm-select-all {
  font: 10px/1 'JetBrains Mono', monospace;
  background: none;
  border: 1px solid var(--panel-edge);
  color: var(--muted);
  border-radius: 3px;
  padding: 3px 8px;
  cursor: pointer;
}
.imm-select-all:hover { color: var(--fg); border-color: var(--accent); }
.imm-sel-count {
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--muted);
}

/* Image grid */
.imm-img-grid {
  position: relative;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well) 40%, transparent);
}
.imm-done-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  pointer-events: none;
}
.imm-done-overlay span {
  font: 900 28px/1.1 'Oswald', sans-serif;
  letter-spacing: .12em;
  text-transform: uppercase;
  color: rgba(180, 180, 180, 0.28);
}
.imm-img-cell {
  position: relative;
  aspect-ratio: 16/9;
  border: 2px solid transparent;
  border-radius: 3px;
  overflow: hidden;
  cursor: pointer;
  background: none;
  padding: 0;
  transition: border-color .12s;
}
.imm-img-cell--selected { border-color: var(--accent); }
.imm-img-cell--assigned { opacity: 0.2; }
.imm-thumb {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.imm-img-check {
  position: absolute;
  top: 2px; right: 3px;
  font: bold 11px/1 'JetBrains Mono', monospace;
  color: var(--accent);
  text-shadow: 0 0 3px rgba(0,0,0,0.8);
}
.imm-img-check--done { color: var(--muted); }

/* Car required gate */
.imm-car-gate {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 6px 10px;
  margin-bottom: 6px;
  background: color-mix(in srgb, #c94444 14%, transparent);
  border: 1px solid #c94444;
  border-radius: 4px;
}
.imm-car-gate-msg {
  font: 11px/1.4 'JetBrains Mono', monospace;
  color: #e07070;
}
.imm-car-gate-dismiss {
  font: 10px/1 'JetBrains Mono', monospace;
  padding: 3px 8px;
  border-radius: 3px;
  border: 1px solid #c94444;
  background: transparent;
  color: #e07070;
  cursor: pointer;
  flex-shrink: 0;
}
.imm-row--highlight .imm-label { color: #e07070; }

/* Assignment controls */
.imm-assign {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--panel-edge);
}
.imm-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.imm-label {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: .08em;
  text-transform: uppercase;
  color: var(--muted);
  min-width: 42px;
  padding-top: 6px;
  flex-shrink: 0;
}
.imm-livery-input {
  flex: 1;
  font: 12px/1 'JetBrains Mono', monospace;
  padding: 5px 8px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well) 60%, transparent);
  color: var(--fg);
  outline: none;
}
.imm-livery-input:focus { border-color: var(--accent); }
.imm-livery-input::placeholder { color: var(--muted); }


/* Actions */
.imm-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  gap: 8px;
}
.imm-nav-btns {
  display: flex;
  gap: 4px;
}
.imm-btn-primary {
  font: 11px/1 'JetBrains Mono', monospace;
  padding: 7px 18px;
  border-radius: 4px;
  border: 1px solid var(--accent);
  background: var(--accent);
  color: #000;
  cursor: pointer;
}
.imm-btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.imm-btn-skip {
  font: 11px/1 'JetBrains Mono', monospace;
  padding: 7px 12px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
}
.imm-btn-skip:hover { color: var(--fg); }
.imm-btn-skip--ready {
  border-color: var(--accent);
  color: var(--accent);
  font-weight: bold;
}
.imm-btn-skip--ready:hover { background: color-mix(in srgb, var(--accent) 12%, transparent); }

.imm-done {
  padding: 24px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}
.imm-done-msg {
  font: 13px/1.4 'JetBrains Mono', monospace;
  color: var(--fg);
  text-align: center;
}
.imm-done-all-clear {
  font: 11px/1 'JetBrains Mono', monospace;
  color: #4a9;
  margin: 0;
}

.imm-retry-panel {
  width: 100%;
  border: 1px solid color-mix(in srgb, #c44 30%, transparent);
  border-radius: 6px;
  overflow: hidden;
}
.imm-retry-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: color-mix(in srgb, #c44 10%, transparent);
  border-bottom: 1px solid color-mix(in srgb, #c44 20%, transparent);
}
.imm-retry-title {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: .06em;
  text-transform: uppercase;
  color: #e07070;
}
.imm-retry-list {
  display: flex;
  flex-direction: column;
}
.imm-retry-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid color-mix(in srgb, var(--panel-edge) 40%, transparent);
}
.imm-retry-row:last-child { border-bottom: none; }
.imm-retry-card {
  font: 9px/1 'JetBrains Mono', monospace;
  color: var(--muted);
  min-width: 80px;
  flex-shrink: 0;
}
.imm-retry-name {
  font: 10px/1 'Oswald', sans-serif;
  color: var(--fg);
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
