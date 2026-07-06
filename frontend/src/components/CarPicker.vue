<template>
  <div class="cp-wrap">
    <!-- Set state: chip (click label area to re-enter search) -->
    <template v-if="car && !searching">
      <span class="cp-chip" @click.self="startSearch(car.game as 'FH5' | 'FH6')">
        <span class="cp-game-badge" @click="startSearch(car.game as 'FH5' | 'FH6')">{{ car.game }}</span>
        <span class="cp-car-label" @click="startSearch(car.game as 'FH5' | 'FH6')">{{ car.year }} {{ car.make }} {{ car.model }}</span>
        <button class="cp-clear" @click="clear" title="Clear car">×</button>
      </span>
    </template>

    <!-- Unset state: game trigger buttons -->
    <template v-else-if="!searching">
      <button class="cp-add-btn" @click="startSearch('FH5')">+ FH5</button>
      <button class="cp-add-btn" @click="startSearch('FH6')">+ FH6</button>
    </template>

    <!-- Search state: input row + drum reel -->
    <template v-else>
      <div class="cp-search-wrap" ref="wrapRef">
        <span class="cp-game-badge cp-game-badge--active">{{ activeGame }}</span>
        <input
          ref="inputRef"
          class="cp-input"
          :placeholder="`${activeGame} — type make or model…`"
          v-model="query"
          @keydown="onKey"
          @input="cursor = -1"
          @blur="onBlur"
          autocomplete="off"
          spellcheck="false"
        />
        <button class="cp-clear" @click="cancel" title="Cancel">×</button>
      </div>

      <!-- Drum reel — Teleported to body to escape modal overflow clipping -->
      <Teleport to="body">
        <div
          v-if="(results.length || query.length > 1) && drumStyle.top !== undefined"
          class="cp-drum"
          :style="drumStyle"
        >
          <ul
            class="cp-drum-track"
            :style="{ transform: `translateY(${trackY}px)` }"
          >
            <li
              v-for="(c, i) in results"
              :key="c.id"
              class="cp-drum-item"
              :class="{ 'cp-drum-item--active': i === cursor }"
              @mousedown.prevent="select(c)"
              @mousemove="cursor = i"
            >
              <span class="cp-opt-year">{{ c.year }}</span>
              <span class="cp-opt-name">{{ c.make }} {{ c.model }}</span>
              <span v-if="c.dlc" class="cp-opt-dlc">{{ c.dlc }}</span>
            </li>
            <li v-if="!results.length" class="cp-drum-item cp-drum-item--none">
              No match — try make or model
            </li>
          </ul>
        </div>
      </Teleport>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useCarsStore } from '../stores/cars'
import type { Car } from '../types'

const ITEM_H  = 28
const N_ABOVE = 3
const N_BELOW = 4
const DRUM_H  = (N_ABOVE + 1 + N_BELOW) * ITEM_H  // 224px

const props = defineProps<{ carId?: string | null }>()
const emit = defineEmits<{ (e: 'update:carId', id: string | null): void }>()

const carsStore = useCarsStore()
carsStore.load()

const searching = ref(false)
const activeGame = ref<'FH5' | 'FH6'>('FH6')
const query = ref('')
const cursor = ref(-1)
const inputRef = ref<HTMLInputElement>()
const wrapRef = ref<HTMLElement>()

const drumStyle = ref<Record<string, string>>({})

function updateDrumStyle() {
  if (!wrapRef.value) return
  const rect = wrapRef.value.getBoundingClientRect()
  drumStyle.value = {
    top:    `${rect.top - N_ABOVE * ITEM_H}px`,
    left:   `${rect.left}px`,
    width:  `${rect.width}px`,
    height: `${DRUM_H}px`,
  }
}

const car = computed<Car | undefined>(() =>
  props.carId ? carsStore.byId(props.carId) : undefined
)

const results = computed<Car[]>(() =>
  searching.value ? carsStore.search(activeGame.value, query.value) : []
)

// Translate the track so the cursor item sits at the center slot.
const trackY = computed(() => {
  const idx = cursor.value >= 0 ? cursor.value : 0
  return N_ABOVE * ITEM_H - idx * ITEM_H
})

async function startSearch(game: 'FH5' | 'FH6') {
  activeGame.value = game
  searching.value = true
  query.value = car.value ? car.value.make : ''
  cursor.value = -1
  await nextTick()
  updateDrumStyle()
  inputRef.value?.focus()
  inputRef.value?.select()
}

function select(c: Car) {
  emit('update:carId', c.id)
  searching.value = false
  query.value = ''
}

function clear() {
  emit('update:carId', null)
}

function cancel() {
  searching.value = false
  query.value = ''
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') { cancel(); return }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    cursor.value = Math.min(cursor.value + 1, results.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    cursor.value = Math.max(cursor.value - 1, 0)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const pick = results.value[cursor.value] ?? results.value[0]
    if (pick) select(pick)
  }
}

function onBlur() {
  setTimeout(() => {
    if (!wrapRef.value?.contains(document.activeElement)) cancel()
  }, 150)
}

watch(() => props.carId, () => {
  if (props.carId) searching.value = false
})
</script>

<style scoped>
.cp-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

/* trigger buttons */
.cp-add-btn {
  font: 11px/1 'Oswald', sans-serif;
  letter-spacing: 0.04em;
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #555);
  background: transparent;
  color: var(--text-muted, #888);
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.cp-add-btn:hover {
  border-color: var(--accent, #c9aa71);
  color: var(--accent, #c9aa71);
}

/* game badge */
.cp-game-badge {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  padding: 2px 5px;
  border-radius: 3px;
  background: var(--muted-light, #444);
  color: var(--text-muted, #aaa);
  white-space: nowrap;
}
.cp-game-badge--active {
  background: var(--accent, #c9aa71);
  color: #000;
}

/* chip (set state) */
.cp-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 6px 3px 4px;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #444);
  background: color-mix(in srgb, var(--panel-well, #1a1a1a) 60%, transparent);
}
.cp-car-label {
  font: 12px/1.2 'Oswald', sans-serif;
  color: var(--text-primary, #e0e0e0);
  cursor: pointer;
}
.cp-car-label:hover { color: var(--accent); }

/* clear × */
.cp-clear {
  font: 14px/1 monospace;
  background: none;
  border: none;
  color: var(--text-muted, #888);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
}
.cp-clear:hover { color: var(--text-primary, #e0e0e0); }

/* search row */
.cp-search-wrap {
  position: relative;
  display: flex;
  align-items: center;
  gap: 5px;
  flex: 1;
  min-width: 220px;
}

.cp-input {
  flex: 1;
  font: 12px/1 'Oswald', sans-serif;
  padding: 4px 6px;
  border-radius: 4px;
  border: 1px solid var(--accent, #c9aa71);
  background: color-mix(in srgb, var(--panel-well, #1a1a1a) 80%, transparent);
  color: var(--text-primary, #e0e0e0);
  outline: none;
  min-width: 0;
}
.cp-input::placeholder { color: var(--text-muted, #666); }

/* drum reel — fixed position, escapes overflow clipping */
.cp-drum {
  position: fixed;
  z-index: 9999;
  overflow: hidden;
  background: color-mix(in srgb, var(--panel, #1e1e1e) 96%, transparent);
  border: 1px solid var(--muted-light, #444);
  border-radius: 4px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.6);
  /* Fade items at top and bottom edges */
  -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 18%, black 82%, transparent 100%);
  mask-image: linear-gradient(to bottom, transparent 0%, black 18%, black 82%, transparent 100%);
}

/* Center slot highlight band */
.cp-drum::before {
  content: '';
  position: absolute;
  left: 0; right: 0;
  top: calc(3 * 28px);  /* N_ABOVE * ITEM_H */
  height: 28px;         /* ITEM_H */
  background: color-mix(in srgb, var(--accent, #c9aa71) 10%, transparent);
  border-top: 1px solid color-mix(in srgb, var(--accent, #c9aa71) 25%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--accent, #c9aa71) 25%, transparent);
  pointer-events: none;
  z-index: 1;
}

.cp-drum-track {
  position: absolute;
  left: 0; right: 0;
  top: 0;
  list-style: none;
  margin: 0;
  padding: 0;
  transition: transform 0.1s ease;
}

.cp-drum-item {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 28px;
  padding: 0 10px;
  cursor: pointer;
  font-size: 12px;
  color: var(--muted, #999);
  font-family: 'Oswald', sans-serif;
}
.cp-drum-item:hover,
.cp-drum-item--active {
  color: var(--fg, #e0e0e0);
}
.cp-drum-item--none {
  color: var(--muted, #666);
  font-style: italic;
  cursor: default;
}

.cp-opt-year {
  font-weight: 700;
  color: var(--muted, #999);
  min-width: 36px;
}
.cp-opt-name { flex: 1; }
.cp-opt-dlc {
  font-size: 10px;
  color: var(--accent, #c9aa71);
  opacity: 0.7;
  white-space: nowrap;
}
</style>
