<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { useLiveriesStore } from '../stores/liveries'
import { useAuthStore } from '../stores/auth'
import { useCarsStore } from '../stores/cars'
import { api } from '../api'
import CarPicker from './CarPicker.vue'
import type { Card } from '../types'

const store = useCardsStore()
const modal = useModalStore()
const liveriesStore = useLiveriesStore()
const auth = useAuthStore()
const carsStore = useCarsStore()
carsStore.load()

const cardsWithImages = computed(() =>
  store.cards.filter(c => !c.isLegend && c.images.length > 0)
)

const currentIndex = ref(0)
const currentCard = computed<Card | null>(() => cardsWithImages.value[currentIndex.value] ?? null)
const done = computed(() => currentIndex.value >= cardsWithImages.value.length)

// Per-image selection
const selectedIds = ref<Set<number>>(new Set())
const assignedIds = ref<Set<number>>(new Set()) // images already given a livery this session

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
const showCarRequired = ref(false)   // blocking gate shown when assign clicked without car
const liveryNameValid = computed(() => liveryName.value.trim().length > 0)
const canAssign = computed(() =>
  selectedIds.value.size > 0 && liveryNameValid.value && !batchProcessing.value
)

// Batch log — multiple batches per card
interface BatchEntry { label: string; count: number; status: 'processing' | 'done' | 'error'; colors?: string; errorMsg?: string }
const batchLog = ref<BatchEntry[]>([])
const batchProcessing = ref(false)

// Reset when card changes
watch(currentCard, (card) => {
  selectedIds.value = new Set()
  assignedIds.value = new Set()
  batchLog.value = []
  batchProcessing.value = false
  carId.value = card?.carId ?? null
  liveryName.value = card?.name?.trim() || ''
}, { immediate: true })

watch(() => modal.imageMigrationOpen, (open) => {
  if (open) currentIndex.value = 0
})

async function assignSelected() {
  const card = currentCard.value
  if (!card || !auth.isAuthenticated || !canAssign.value) return

  // Car is required — show blocking gate instead of proceeding.
  if (!carId.value) {
    showCarRequired.value = true
    return
  }

  const ids = [...selectedIds.value]
  const name = liveryName.value.trim()

  batchProcessing.value = true
  batchLog.value.push({ label: name, count: ids.length, status: 'processing' })
  const entry = batchLog.value[batchLog.value.length - 1]

  try {
    // Create livery first so we have the id for migrate.
    const livery = await liveriesStore.create({ carId: carId.value, name })
    if (!livery) throw new Error('Failed to create livery')

    // Re-file the images on disk with new naming scheme, update DB rows.
    const result = await api.migrateImages(ids, carId.value, livery.id)

    // Reflect new paths in the card store so the UI updates without reload.
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

    // Mark as assigned so they dim.
    const next = new Set(assignedIds.value)
    ids.forEach(id => next.add(id))
    assignedIds.value = next

    // Ready next batch.
    selectedIds.value = new Set()
    carId.value = null
    liveryName.value = currentCard.value?.name?.trim() || ''

    entry.status = 'done'

    // Assess color in background — updates entry when done.
    api.assessLiveryColor(livery.id)
      .then(r => { entry.colors = r.primary + (r.secondary ? ' / ' + r.secondary : '') })
      .catch(() => {})
  } catch (e) {
    entry.status = 'error'
    entry.errorMsg = e instanceof Error ? e.message : String(e)
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
            <span class="imm-counter">{{ currentIndex + 1 }} / {{ cardsWithImages.length }}</span>
            <h2 class="imm-title">{{ currentCard.name }}</h2>
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
          <div class="imm-img-grid">
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

          <!-- Batch log -->
          <div v-if="batchLog.length" class="imm-batch-log">
            <div
              v-for="(b, i) in batchLog"
              :key="i"
              class="imm-batch-row"
              :class="'imm-batch-row--' + b.status"
            >
              <span class="imm-batch-label">{{ b.label }}</span>
              <span class="imm-batch-meta">
                {{ b.count }} photo{{ b.count !== 1 ? 's' : '' }}
                <template v-if="b.status === 'processing'"> · saving…</template>
                <template v-else-if="b.colors"> · {{ b.colors }}</template>
                <template v-else-if="b.status === 'error'"> · {{ b.errorMsg || 'failed' }}</template>
              </span>
            </div>
          </div>

          <!-- Actions -->
          <div class="imm-actions">
            <button class="imm-btn-skip" @click="nextCard">Next card →</button>
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
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--panel-edge);
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
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel-well) 40%, transparent);
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
.imm-img-cell--assigned { opacity: 0.45; }
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

/* Batch log */
.imm-batch-log {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 14px;
  border-bottom: 1px solid var(--panel-edge);
}
.imm-batch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font: 11px/1.4 'JetBrains Mono', monospace;
  padding: 2px 0;
}
.imm-batch-label { color: var(--fg); }
.imm-batch-meta { color: var(--muted); font-size: 10px; }
.imm-batch-row--done .imm-batch-label { color: var(--accent); }
.imm-batch-row--processing .imm-batch-label { color: var(--muted); font-style: italic; }
.imm-batch-row--error .imm-batch-label { color: #c94444; }

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
