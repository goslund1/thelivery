<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import type { AdjustmentRow } from '../types'
import { api } from '../api'
import { useCardsStore } from '../stores/cards'
import { useCarsStore } from '../stores/cars'
import { useModalStore } from '../stores/modal'
import TuningAdjustments from './TuningAdjustments.vue'

const emit = defineEmits<{ close: [] }>()

const cardsStore = useCardsStore()
const carsStore  = useCarsStore()
const modal      = useModalStore()
const promoBusy  = ref(false)
const promoError = ref<string | null>(null)

type Suggestion = {
  id: number
  cardId: string
  title: string
  credit: string | null
  adjustments: AdjustmentRow[]
  submittedAt: string
  ip: string
  status: 'pending' | 'liked'
}

const suggestions  = ref<Suggestion[]>([])
const loading      = ref(false)
const error        = ref<string | null>(null)
const activeTab    = ref<'pending' | 'liked'>('pending')
const selectedId   = ref<number | null>(null)
const actionBusy   = ref(false)

async function load() {
  loading.value = true
  error.value = null
  try {
    suggestions.value = (await api.adminListSuggestions()) as Suggestion[]
  } catch (e) {
    error.value = (e as Error).message
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  document.body.style.overflow = 'hidden'
  load()
})
onUnmounted(() => { document.body.style.overflow = '' })

const visible = computed(() =>
  suggestions.value.filter(s => s.status === activeTab.value)
)

// Auto-select first visible suggestion when list or tab changes.
watch(visible, (list) => {
  if (!list.some(s => s.id === selectedId.value)) {
    selectedId.value = list[0]?.id ?? null
  }
}, { immediate: true })

const current = computed(() =>
  suggestions.value.find(s => s.id === selectedId.value) ?? null
)

const card = computed(() =>
  current.value ? cardsStore.byId(current.value.cardId) : null
)

const car = computed(() => {
  const c = card.value
  if (!c?.carId) return null
  return carsStore.byId(c.carId) ?? null
})

// Card's current recipe adjustments — used as baseline for diff highlighting.
const cardAdjustments = computed((): AdjustmentRow[] => {
  if (!card.value) return []
  const recipe = card.value.sections.find(s => s.type === 'forza_recipe')
  return recipe?.type === 'forza_recipe' ? recipe.adjustments : []
})

const cardUpgrades = computed(() => {
  if (!card.value) return []
  const recipe = card.value.sections.find(s => s.type === 'forza_recipe')
  return recipe?.type === 'forza_recipe' ? recipe.upgrades : []
})

function carLabel(c: typeof car.value) {
  if (!c) return ''
  return `${c.year ?? ''} ${c.make} ${c.model}`.trim()
}

function fmtDate(iso: string) {
  return iso.slice(0, 10)
}

function advanceAfter(id: number) {
  const list = visible.value
  const idx  = list.findIndex(s => s.id === id)
  const next = list[idx + 1] ?? list[idx - 1] ?? null
  selectedId.value = next?.id ?? null
}

async function onDismiss() {
  if (!current.value || actionBusy.value) return
  const id = current.value.id
  const wasPending = current.value.status === 'pending'
  actionBusy.value = true
  try {
    await api.adminDismissSuggestion(id)
    advanceAfter(id)
    suggestions.value = suggestions.value.filter(s => s.id !== id)
    if (wasPending) modal.pendingSuggestionCount = Math.max(0, modal.pendingSuggestionCount - 1)
  } finally {
    actionBusy.value = false
  }
}

async function onLike() {
  if (!current.value || actionBusy.value) return
  const id = current.value.id
  const wasLiked = current.value.status === 'liked'
  actionBusy.value = true
  try {
    await api.adminLikeSuggestion(id)
    const s = suggestions.value.find(s => s.id === id)
    if (s) {
      if (!wasLiked) {
        advanceAfter(id)
        modal.pendingSuggestionCount = Math.max(0, modal.pendingSuggestionCount - 1)
      } else {
        modal.pendingSuggestionCount++
      }
      s.status = wasLiked ? 'pending' : 'liked'
    }
  } finally {
    actionBusy.value = false
  }
}

async function onPromote() {
  if (!current.value || promoBusy.value) return
  promoBusy.value = true
  promoError.value = null
  try {
    const promoted = await cardsStore.promoteCard(current.value.cardId, {
      adjustments: current.value.adjustments,
    })
    modal.openPromotedCard(promoted)
  } catch (e) {
    promoError.value = (e as Error).message
  } finally {
    promoBusy.value = false
  }
}

function onOverlay(e: MouseEvent) {
  if (e.target === e.currentTarget) emit('close')
}
</script>

<template>
  <Teleport to="body">
    <!-- TODO: remove legacy class sv-backdrop when float_ system is complete -->
    <div class="sv-backdrop float_suggestions_backdrop" @click="onOverlay">
      <!-- TODO: remove legacy class sv-modal when float_ system is complete -->
      <div class="sv-modal float_suggestions_panel">

        <!-- Header -->
        <div class="sv-header">
          <div class="sv-header-left">
            <span class="sv-header-title">Tune Suggestions</span>
            <span v-if="current" class="sv-header-card">
              {{ card?.name ?? current.cardId }}
              <span v-if="car" class="sv-header-car"> · {{ carLabel(car) }}</span>
            </span>
          </div>
          <button class="sv-close" @click="emit('close')">✕</button>
        </div>

        <!-- Tabs + selector -->
        <div class="sv-controls">
          <div class="sv-tabs">
            <button
              class="sv-tab"
              :class="{ active: activeTab === 'pending' }"
              @click="activeTab = 'pending'"
            >Pending <span v-if="suggestions.filter(s => s.status === 'pending').length" class="sv-count">{{ suggestions.filter(s => s.status === 'pending').length }}</span></button>
            <button
              class="sv-tab"
              :class="{ active: activeTab === 'liked' }"
              @click="activeTab = 'liked'"
            >Liked <span v-if="suggestions.filter(s => s.status === 'liked').length" class="sv-count">{{ suggestions.filter(s => s.status === 'liked').length }}</span></button>
          </div>

          <select v-if="visible.length" class="sv-select" :value="selectedId" @change="selectedId = Number(($event.target as HTMLSelectElement).value)">
            <option v-for="s in visible" :key="s.id" :value="s.id">
              {{ cardsStore.byId(s.cardId)?.name ?? s.cardId }} · {{ s.credit ?? 'anonymous' }} · {{ fmtDate(s.submittedAt) }}
            </option>
          </select>
        </div>

        <!-- Body -->
        <div class="sv-body">
          <div v-if="loading" class="sv-state">Loading…</div>
          <div v-else-if="error" class="sv-state sv-state--error">{{ error }}</div>
          <div v-else-if="!visible.length" class="sv-state sv-state--empty">
            No {{ activeTab }} suggestions.
          </div>

          <template v-else-if="current">
            <!-- Suggestion meta -->
            <div class="sv-meta">
              <span class="sv-meta-title">{{ current.title }}</span>
              <span v-if="current.credit" class="sv-meta-credit">by {{ current.credit }}</span>
              <span class="sv-meta-date">{{ fmtDate(current.submittedAt) }}</span>
            </div>

            <!-- Actions -->
            <div class="sv-actions">
              <button
                class="sv-btn sv-btn--like"
                :class="{ active: current.status === 'liked' }"
                :disabled="actionBusy || promoBusy"
                @click="onLike"
              >{{ current.status === 'liked' ? '★ Liked' : '☆ Like' }}</button>
              <button
                class="sv-btn sv-btn--promote"
                :disabled="actionBusy || promoBusy"
                @click="onPromote"
              >{{ promoBusy ? '…' : 'Promote' }}</button>
              <button
                class="sv-btn sv-btn--dismiss"
                :disabled="actionBusy || promoBusy"
                @click="onDismiss"
              >Dismiss</button>
            </div>
            <p v-if="promoError" class="sv-promo-error">{{ promoError }}</p>

            <!-- Tuning widget — read-only, diffed against card's current values -->
            <TuningAdjustments
              v-if="current.adjustments.length"
              :adjustments="current.adjustments"
              :baseline-adjustments="cardAdjustments"
              :upgrades="cardUpgrades"
              :card-id="current.cardId"
              :read-only="true"
            />
            <div v-else class="sv-state sv-state--empty">No adjustment data in this suggestion.</div>
          </template>
        </div>

      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.sv-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.55);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  overflow-y: auto;
  padding: 24px 16px 40px;
}
.sv-modal {
  background: var(--glass-bg);
  backdrop-filter: blur(18px) saturate(1.4);
  -webkit-backdrop-filter: blur(18px) saturate(1.4);
  border: 1px solid var(--panel-edge);
  border-radius: 8px;
  box-shadow: 0 8px 40px rgba(0,0,0,.4);
  width: 100%;
  max-width: 860px;
  min-height: 300px;
  display: flex;
  flex-direction: column;
}

/* Header */
.sv-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 12px;
  border-bottom: 1px solid var(--panel-edge);
  gap: 12px;
}
.sv-header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.sv-header-title {
  font-family: 'Oswald', sans-serif;
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: .08em;
  color: var(--muted);
}
.sv-header-card {
  font-family: 'Oswald', sans-serif;
  font-size: 18px;
  font-weight: 600;
  color: var(--highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sv-header-car {
  color: var(--muted);
  font-weight: 400;
  font-size: 15px;
}
.sv-close {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 18px;
  cursor: pointer;
  line-height: 1;
  padding: 4px 6px;
  border-radius: 4px;
  transition: color .12s;
  flex-shrink: 0;
}
.sv-close:hover { color: var(--fg); }

/* Controls */
.sv-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--panel-edge);
  flex-wrap: wrap;
}
.sv-tabs {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.sv-tab {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .06em;
  padding: 4px 10px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 5px;
  transition: border-color .12s, color .12s;
}
.sv-tab.active { border-color: var(--accent); color: var(--accent); }
.sv-tab:not(.active):hover { color: var(--fg); border-color: var(--fg); }
.sv-count {
  background: var(--accent);
  color: var(--bg);
  border-radius: 8px;
  font-size: 10px;
  padding: 0 5px;
  font-weight: bold;
}
.sv-select {
  flex: 1;
  min-width: 200px;
  background: var(--panel-well, var(--glass-bg));
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 5px 8px;
  cursor: pointer;
}

/* Body */
.sv-body {
  flex: 1;
  padding: 16px 20px 24px;
  overflow-y: auto;
}
.sv-state {
  color: var(--muted);
  font-size: 13px;
  font-family: 'JetBrains Mono', monospace;
  padding: 32px 0;
  text-align: center;
}
.sv-state--error { color: #e05c5c; }
.sv-state--empty { }

/* Meta + actions */
.sv-meta {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}
.sv-meta-title {
  font-family: 'Oswald', sans-serif;
  font-size: 16px;
  font-weight: 600;
  color: var(--highlight);
}
.sv-meta-credit {
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--accent);
}
.sv-meta-date {
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--muted);
  margin-left: auto;
}

.sv-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
}
.sv-btn {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .06em;
  padding: 5px 14px;
  cursor: pointer;
  transition: border-color .12s, color .12s;
}
.sv-btn:disabled { opacity: .4; cursor: default; }
.sv-btn--like:hover, .sv-btn--like.active { border-color: var(--accent); color: var(--accent); }
.sv-btn--promote { margin-left: auto; }
.sv-btn--promote:hover { border-color: #2d6a3f; color: #a8d8b0; background: rgba(45,106,63,.12); }
.sv-btn--dismiss:hover { border-color: #e05c5c; color: #e05c5c; }
.sv-promo-error {
  font-size: 11px;
  color: #e05c5c;
  margin: -12px 0 16px;
  font-family: 'JetBrains Mono', monospace;
}
</style>
