<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { useLiveriesStore } from '../stores/liveries'
import { useAuthStore } from '../stores/auth'
import { api } from '../api'
import CarPicker from './CarPicker.vue'
import type { Card } from '../types'

const store = useCardsStore()
const modal = useModalStore()
const liveriesStore = useLiveriesStore()
const auth = useAuthStore()

// Walk through cards that have images
const cardsWithImages = computed(() =>
  store.cards.filter(c => !c.isLegend && c.images.length > 0)
)

const currentIndex = ref(0)
const currentCard = computed<Card | null>(() => cardsWithImages.value[currentIndex.value] ?? null)
const done = computed(() => currentIndex.value >= cardsWithImages.value.length)

// Per-card state
const carId = ref<string | null>(null)
const liveryName = ref('')
const liveryNameValid = computed(() => liveryName.value.trim().length > 0 && liveryName.value.trim() !== 'Livery Name')

// Import log per card
interface LogEntry { label: string; progress: number; status: 'pending' | 'assessing' | 'done' | 'error' }
const log = ref<LogEntry[]>([])
const assessStatus = ref<'idle' | 'pending' | 'done' | 'error'>('idle')
const assessColors = ref<{ primary: string; secondary?: string } | null>(null)
const processing = ref(false)
const logFading = ref(false)
let fadeTimer: ReturnType<typeof setTimeout> | null = null

// Reset per-card state when card changes
watch(currentCard, (card) => {
  carId.value = card?.carId ?? null
  liveryName.value = card ? (card.name.trim() || 'Livery Name') : ''
  log.value = []
  assessStatus.value = 'idle'
  assessColors.value = null
  processing.value = false
  logFading.value = false
  if (fadeTimer) { clearTimeout(fadeTimer); fadeTimer = null }
}, { immediate: true })

watch(() => modal.imageMigrationOpen, (open) => {
  if (open) { currentIndex.value = 0 }
})

async function processCard() {
  const card = currentCard.value
  if (!card || !auth.isAuthenticated) return
  if (!liveryNameValid.value) return

  processing.value = true
  assessStatus.value = 'idle'
  assessColors.value = null

  // Build log entries — one per image
  log.value = card.images
    .slice()
    .sort((a, b) => a.order - b.order)
    .map(img => ({
      label: img.path.split('/').pop() ?? img.path,
      progress: 0,
      status: 'pending' as const,
    }))

  // Create livery
  const livery = carId.value
    ? await liveriesStore.create({ carId: carId.value, name: liveryName.value.trim() })
    : null

  if (livery) assessStatus.value = 'pending'

  // Register all images via assess endpoint trigger.
  // Images are already in the DB (sync_card_images ran on load), so we just
  // need to set livery_id on them and trigger assess.
  // We do this by patching each image's livery_id via card save.
  const sortedImages = card.images.slice().sort((a, b) => a.order - b.order)

  for (let i = 0; i < sortedImages.length; i++) {
    const img = sortedImages[i]
    if (img && livery) {
      store.setImageMeta(card.id, img.id, { carId: carId.value ?? undefined, liveryId: livery.id })
    }
    log.value[i] = { ...log.value[i], progress: 100, status: 'done' }
  }

  // Save card so image livery_id rows get updated
  await store.save(card.id)

  // Trigger assess if livery was created
  if (livery) {
    api.assessLiveryColor(livery.id)
      .then(r => {
        assessStatus.value = 'done'
        assessColors.value = { primary: r.primary, secondary: r.secondary }
      })
      .catch(() => { assessStatus.value = 'error' })
  } else {
    assessStatus.value = 'idle'
  }

  // Wait briefly then auto-advance
  fadeTimer = setTimeout(() => {
    logFading.value = true
    fadeTimer = setTimeout(() => {
      currentIndex.value++
      processing.value = false
    }, 700)
  }, livery ? 4000 : 1500) // longer wait when assessing
}

function skip() {
  if (fadeTimer) { clearTimeout(fadeTimer); fadeTimer = null }
  logFading.value = false
  log.value = []
  assessStatus.value = 'idle'
  processing.value = false
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
          <p class="imm-done-msg">All {{ cardsWithImages.length }} cards processed.</p>
          <button class="imm-btn-primary" @click="close">Close</button>
        </div>

        <template v-else-if="currentCard">
          <div class="imm-header">
            <span class="imm-counter">{{ currentIndex + 1 }} / {{ cardsWithImages.length }}</span>
            <h2 class="imm-title">{{ currentCard.name }}</h2>
          </div>

          <!-- Image preview strip -->
          <div class="imm-img-strip">
            <img
              v-for="img in currentCard.images.slice().sort((a,b) => a.order - b.order)"
              :key="img.id"
              :src="img.thumbPath ?? img.path"
              class="imm-thumb"
              :alt="img.alt ?? ''"
            />
          </div>

          <!-- Setup: car + livery name -->
          <div v-if="!processing" class="imm-setup">
            <div class="imm-row">
              <span class="imm-label">Car</span>
              <CarPicker :car-id="carId" @update:car-id="id => carId = id" />
            </div>
            <div class="imm-row">
              <span class="imm-label">Livery</span>
              <input
                class="imm-livery-input"
                :class="{ 'imm-livery-input--default': !liveryNameValid }"
                v-model="liveryName"
                placeholder="Unique livery name…"
              />
            </div>
          </div>

          <!-- Progress log -->
          <div v-if="log.length" class="imm-log" :class="{ 'imm-log--fading': logFading }">
            <div
              v-for="(entry, i) in log"
              :key="i"
              class="imm-log-row"
              :class="'imm-log-row--' + entry.status"
              :style="{ '--prog': entry.progress + '%' }"
            >
              <span class="imm-log-label">{{ entry.label }}</span>
              <span class="imm-log-status">{{ entry.status === 'done' ? '✓' : entry.status === 'error' ? '✗' : '…' }}</span>
            </div>
            <div v-if="assessStatus !== 'idle'" class="imm-log-row" :class="assessStatus === 'pending' ? 'imm-log-row--pending' : assessStatus === 'done' ? 'imm-log-row--done' : 'imm-log-row--error'" :style="{ '--prog': assessStatus === 'pending' ? '55%' : '100%' }">
              <span class="imm-log-label">Color assess</span>
              <span class="imm-log-status">
                <template v-if="assessStatus === 'pending'">assessing…</template>
                <template v-else-if="assessStatus === 'done' && assessColors">{{ assessColors.primary }}<template v-if="assessColors.secondary"> / {{ assessColors.secondary }}</template></template>
                <template v-else>failed</template>
              </span>
            </div>
          </div>

          <!-- Actions -->
          <div v-if="!processing" class="imm-actions">
            <button class="imm-btn-skip" @click="skip">Skip →</button>
            <button
              class="imm-btn-primary"
              :disabled="!liveryNameValid"
              @click="processCard"
            >Process card →</button>
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
  width: min(92vw, 580px);
  max-height: 88vh;
  overflow-y: auto;
  background: var(--panel-bg, #1a1a1a);
  border: 1px solid var(--panel-edge, #333);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 0;
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
  padding: 16px 16px 8px;
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

.imm-img-strip {
  display: flex;
  gap: 4px;
  padding: 10px 14px;
  overflow-x: auto;
  background: color-mix(in srgb, var(--panel-well) 40%, transparent);
  border-bottom: 1px solid var(--panel-edge);
}
.imm-thumb {
  height: 72px;
  width: auto;
  border-radius: 3px;
  object-fit: cover;
  flex-shrink: 0;
  border: 1px solid var(--panel-edge);
}

.imm-setup {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
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
.imm-livery-input--default { color: var(--muted); }

.imm-log {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 14px;
  opacity: 1;
  transition: opacity .7s ease;
}
.imm-log--fading { opacity: 0; }

.imm-log-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-radius: 3px;
  font: 11px/1 'JetBrains Mono', monospace;
  overflow: hidden;
}
.imm-log-row::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to right,
    color-mix(in srgb, var(--accent) 18%, transparent) var(--prog, 0%),
    transparent var(--prog, 0%)
  );
  pointer-events: none;
}
.imm-log-label { color: var(--muted); position: relative; }
.imm-log-status { flex-shrink: 0; position: relative; }
.imm-log-row--done .imm-log-label,
.imm-log-row--done .imm-log-status { color: var(--accent); }
.imm-log-row--error .imm-log-label,
.imm-log-row--error .imm-log-status { color: #c94444; }
.imm-log-row--pending .imm-log-status { color: var(--muted); font-style: italic; }

.imm-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 14px;
  border-top: 1px solid var(--panel-edge);
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
.imm-btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
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
