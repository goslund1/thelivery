<template>
  <div class="cp-wrap">
    <!-- Set state: chip -->
    <template v-if="car && !searching">
      <span class="cp-chip">
        <span class="cp-game-badge">{{ car.game }}</span>
        <span class="cp-car-label">{{ car.year }} {{ car.make }} {{ car.model }}</span>
        <button class="cp-clear" @click="clear" title="Clear car">×</button>
      </span>
    </template>

    <!-- Unset state: game trigger buttons -->
    <template v-else-if="!searching">
      <button class="cp-add-btn" @click="startSearch('FH5')">+ FH5</button>
      <button class="cp-add-btn" @click="startSearch('FH6')">+ FH6</button>
    </template>

    <!-- Search state: input + dropdown -->
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
        <ul v-if="results.length" class="cp-dropdown" ref="listRef">
          <li
            v-for="(c, i) in results"
            :key="c.id"
            class="cp-option"
            :class="{ 'cp-option--active': i === cursor }"
            @mousedown.prevent="select(c)"
            @mousemove="cursor = i"
          >
            <span class="cp-opt-year">{{ c.year }}</span>
            <span class="cp-opt-name">{{ c.make }} {{ c.model }}</span>
            <span v-if="c.dlc" class="cp-opt-dlc">{{ c.dlc }}</span>
          </li>
        </ul>
        <ul v-else-if="query.length > 1" class="cp-dropdown cp-dropdown--empty">
          <li class="cp-option cp-option--none">No match — try make or model</li>
        </ul>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useCarsStore } from '../stores/cars'
import type { Car } from '../types'

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
const listRef = ref<HTMLElement>()

const car = computed<Car | undefined>(() =>
  props.carId ? carsStore.byId(props.carId) : undefined
)

const results = computed<Car[]>(() =>
  searching.value ? carsStore.search(activeGame.value, query.value) : []
)

async function startSearch(game: 'FH5' | 'FH6') {
  activeGame.value = game
  searching.value = true
  query.value = ''
  cursor.value = -1
  await nextTick()
  inputRef.value?.focus()
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
    scrollCursor()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    cursor.value = Math.max(cursor.value - 1, 0)
    scrollCursor()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const pick = results.value[cursor.value] ?? results.value[0]
    if (pick) select(pick)
  }
}

function scrollCursor() {
  nextTick(() => {
    const li = listRef.value?.children[cursor.value] as HTMLElement | undefined
    li?.scrollIntoView({ block: 'nearest' })
  })
}

// short delay lets mousedown.prevent select fire before blur dismisses
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
}

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

/* search wrap */
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

/* dropdown */
.cp-dropdown {
  position: absolute;
  top: calc(100% + 3px);
  left: 0;
  right: 0;
  max-height: 220px;
  overflow-y: auto;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #444);
  background: color-mix(in srgb, var(--panel-bg, #1e1e1e) 95%, transparent);
  list-style: none;
  margin: 0;
  padding: 2px 0;
  z-index: 200;
  box-shadow: 0 4px 16px rgba(0,0,0,0.5);
}
.cp-dropdown--empty { }

.cp-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-secondary, #ccc);
  transition: background 0.08s;
}
.cp-option--active,
.cp-option:hover {
  background: color-mix(in srgb, var(--accent, #c9aa71) 15%, transparent);
  color: var(--text-primary, #e0e0e0);
}
.cp-option--none {
  color: var(--text-muted, #666);
  font-style: italic;
  cursor: default;
}
.cp-opt-year {
  font-weight: 700;
  color: var(--text-muted, #999);
  min-width: 36px;
  font-family: 'Oswald', sans-serif;
}
.cp-opt-name { flex: 1; font-family: 'Oswald', sans-serif; }
.cp-opt-dlc {
  font-size: 10px;
  color: var(--accent, #c9aa71);
  opacity: 0.7;
  white-space: nowrap;
}
</style>
