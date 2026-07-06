<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { useLiveriesStore } from '../stores/liveries'
import { useAuthStore } from '../stores/auth'
import { useCarsStore } from '../stores/cars'
import { useToastsStore } from '../stores/toasts'
import { api } from '../api'
import CarPicker from './CarPicker.vue'
import type { Card } from '../types'

const store = useCardsStore()
const modal = useModalStore()
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

  const toastId = toasts.push(`${card.name} — ${carLabel}`, [
    { text: `${ids.length} image${ids.length !== 1 ? 's' : ''} — re-filing…`, status: 'processing' },
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
    if (!livery) throw new Error('Failed to create livery')
    toasts.updateItem(toastId, liveryItemId, { status: 'done', text: `Livery: ${name}`, detail: livery.serial })

    const result = await api.migrateImages(ids, carId.value!, livery.id)

    for (const updated of result.migrated) {
      store.setImageMeta(card.id, updated.id, {
        carId: updated.carId,
        liveryId: updated.liveryId,
        path: updated.path,
        thumbPath: updated.thumbPath,
        stagePath: updated.stagePath,
      })
    }
    await store.save(card.id)

    toasts.updateItem(toastId, imgItemId, { status: 'done', text: `${result.migrated.length} image${result.migrated.length !== 1 ? 's' : ''} migrated` })

    selectedIds.value = new Set()
    carId.value = null
    liveryName.value = currentCard.value?.name?.trim() || ''

    toasts.updateItem(toastId, assessItemId, { status: 'processing', text: 'Assessing colors…' })
    api.assessLiveryColor(livery.id)
      .then(r => {
        const colors = r.primary + (r.secondary ? ' / ' + r.secondary : '')
        toasts.updateItem(toastId, assessItemId, { status: 'done', text: 'Colors assessed', detail: colors })
      })
      .catch(() => {
        toasts.updateItem(toastId, assessItemId, { status: 'done', text: 'Color assess skipped' })
      })
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    toasts.updateItem(toastId, imgItemId, { status: 'error', text: msg })
    toasts.updateItem(toastId, liveryItemId, { status: 'error', text: 'Failed' })
    toasts.updateItem(toastId, assessItemId, { status: 'error', text: '' })
    console.error('[ImageMigration] assignSelected failed:', e)
  } finally {
    batchProcessing.value = false
  }
}

function nextCard() {
  currentIndex.value++
}

function close() { modal.closeImageMigration() }
</script>

<template>
  <Teleport to="body">
    <div v-if="modal.imageMigrationOpen" class="imm-overlay" @click.self="close">
      <div class="imm-shell">
        <button class="imm-close" @click="close">×</button>

        <div v-if="done" class="imm-done">
          <p class="imm-done-msg">All {{ cardsWithImages.length }} cards visited.</p>
          <button class="imm-btn-primary" @click="close">Close</button>
        </div>

        <template v-else-if="currentCard">
          <!-- Header -->
          <div class="imm-header">
            <div class="imm-header-top">
              <span class="imm-counter">{{ currentIndex + 1 }} / {{ cardsWithImages.length }}</span>
              <h2 class="imm-title">{{ currentCard.name }}</h2>
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
            <!-- Car required gate -->
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
            <button
              class="imm-btn-skip"
              :class="{ 'imm-btn-skip--ready': allAssigned }"
              @click="nextCard"
            >Next card →</button>
            <button
              class="imm-btn-primary"
              :disabled="!canAssign"
              @click="assignSelected"
            >Assign selected →</button>
          </div>
        </template>
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
.imm-shell {
  position: relative;
  width: min(92vw, 600px);
  max-height: 90vh;
  overflow-y: auto;
  background: var(--panel-bg, #1a1a1a);
  border: 1px solid var(--panel-edge, #333);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
}
.imm-close {
  position: absolute;
  top: 8px; right: 10px;
  background: none; border: none;
  color: var(--muted); font-size: 18px;
  cursor: pointer; z-index: 1;
}
.imm-close:hover { color: var(--fg); }

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
  padding: 32px 24px;
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
</style>
