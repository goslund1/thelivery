<script setup lang="ts">
import { ref, watch, nextTick, inject, computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useFactoidSchema } from '../composables/useFactoidSchema'
import { useCardsStore } from '../stores/cards'
import { MarkDirtyKey } from '../keys'

const props = defineProps<{ modelValue: string }>()
const emit = defineEmits<{ 'update:modelValue': [v: string] }>()

const ui        = useUiStore()
const modal     = useModalStore()
const cards     = useCardsStore()
const { schema, addOption } = useFactoidSchema()
const markDirty = inject(MarkDirtyKey, () => {})

type Mode = 'select' | 'editing'

// One entry per visible slot — spliced on remove, pushed on add
const values    = ref<string[]>([])
const modes     = ref<Mode[]>([])
const drafts    = ref<string[]>([])
const inputRefs = ref<(HTMLInputElement | null)[]>([])

let suppressParse = false

function parse(subtitle: string) {
  const parts = subtitle.split(' · ').map(p => p.trim())
  // Show at least schema.length slots so the user can fill them in
  const len   = Math.max(schema.value.length, parts.filter(Boolean).length)
  values.value = Array.from({ length: len }, (_, i) => parts[i] ?? '')
  modes.value  = values.value.map((): Mode => 'select')
  drafts.value = [...values.value]
}

watch([() => props.modelValue, schema], ([v]) => {
  if (!suppressParse) parse(v as string)
}, { immediate: true, deep: true })

function flush() {
  suppressParse = true
  emit('update:modelValue', values.value.filter(s => s.trim()).join(' · '))
  markDirty()
  nextTick(() => { suppressParse = false })
}

// Options for slot i: schema-stored options merged with values seen across cards
function optionsFor(i: number): string[] {
  const schemaOpts = schema.value[i]?.options ?? []
  const cardOpts   = cards.allSubtitleSegments(i)
  return [...new Set([...schemaOpts, ...cardOpts])].filter(Boolean).sort()
}

function labelFor(i: number): string {
  return schema.value[i]?.name ?? 'Extra'
}

const slotCount = computed(() => values.value.length)

function onSelectChange(i: number, e: Event) {
  const sel = e.target as HTMLSelectElement
  const v   = sel.value

  if (v === '__manage__') {
    // Reset select back to current value then open the panel
    sel.value = values.value[i] || ''
    modal.openFactoidPanel()
    return
  }
  if (v === '__add_new__') {
    drafts.value[i] = ''
    modes.value[i]  = 'editing'
    nextTick(() => inputRefs.value[i]?.focus())
    return
  }
  if (v === '__remove__') {
    values.value.splice(i, 1)
    modes.value.splice(i, 1)
    drafts.value.splice(i, 1)
    flush()
    return
  }
  values.value[i] = v
  flush()
}

function confirmEdit(i: number) {
  const val = drafts.value[i].trim()
  values.value[i] = val
  modes.value[i]  = 'select'
  // Persist new value into schema for this slot type
  const type = schema.value[i]
  if (type && val && !type.options.includes(val)) {
    addOption(type.key, val)
  }
  flush()
}

function cancelEdit(i: number) {
  drafts.value[i] = values.value[i]
  modes.value[i]  = 'select'
}

function addSlot() {
  values.value.push('')
  modes.value.push('editing')
  drafts.value.push('')
  flush()
  nextTick(() => inputRefs.value[values.value.length - 1]?.focus())
}
</script>

<template>
  <p v-if="!ui.isEditing" class="card-sub">{{ modelValue }}</p>

  <div v-else class="sub-root">
    <div v-for="i in slotCount" :key="i - 1" class="sub-seg">
      <select
        v-if="modes[i - 1] === 'select'"
        class="sub-select"
        :class="{ 'sub-select--set': !!(values[i - 1]) }"
        @change="onSelectChange(i - 1, $event)"
      >
        <option value="" :selected="!values[i - 1]">{{ labelFor(i - 1) }}</option>
        <option
          v-for="opt in optionsFor(i - 1)"
          :key="opt"
          :value="opt"
          :selected="values[i - 1] === opt"
        >{{ opt }}</option>
        <option value="__add_new__">＋ Add new…</option>
        <option value="__remove__">✕ Remove Factoid</option>
        <option disabled>──────────</option>
        <option value="__manage__">⚙ Manage types…</option>
      </select>

      <div v-else class="sub-edit">
        <input
          :ref="(el) => { inputRefs[i - 1] = el as HTMLInputElement | null }"
          class="sub-edit-input"
          v-model="drafts[i - 1]"
          :placeholder="labelFor(i - 1)"
          @keydown.enter.prevent="confirmEdit(i - 1)"
          @keydown.escape="cancelEdit(i - 1)"
        />
        <button class="sub-confirm" type="button" @click="confirmEdit(i - 1)">＋</button>
      </div>
    </div>

    <!-- Add extra slot -->
    <div class="sub-seg sub-seg--add">
      <button class="sub-add" type="button" title="Add Factoid" @click="addSlot">＋</button>
    </div>

    <!-- Gear: open factoid panel -->
    <div class="sub-seg sub-seg--gear">
      <button class="sub-gear" type="button" title="Manage factoid types" @click="modal.openFactoidPanel()">
        <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="8" cy="8" r="2.5"></circle>
          <path d="M8 1v2M8 13v2M1 8h2M13 8h2M3.05 3.05l1.42 1.42M11.53 11.53l1.42 1.42M3.05 12.95l1.42-1.42M11.53 4.47l1.42-1.42"></path>
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.sub-root {
  display: flex;
  flex-wrap: wrap;
  align-items: stretch;
  margin-bottom: 18px;
}

.sub-seg {
  position: relative;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px 4px 0;
  flex: 1;
  min-width: 80px;
}
.sub-seg::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    to right,
    color-mix(in srgb, var(--panel-edge) 75%, transparent) 0%,
    transparent 68%
  );
  pointer-events: none;
}
.sub-seg--add  { flex: 0 0 auto; min-width: 0; }
.sub-seg--gear { flex: 0 0 auto; min-width: 0; }
.sub-seg--add::after, .sub-seg--gear::after { display: none; }

.sub-select {
  flex: 1;
  background: none;
  border: none;
  color: var(--muted);
  font-family: inherit;
  font-size: 13px;
  padding: 0;
  margin: 0;
  cursor: pointer;
  opacity: 0.65;
  min-width: 0;
  -webkit-appearance: auto;
  appearance: auto;
}
.sub-select:hover  { opacity: 1; }
.sub-select:focus  { outline: 1px solid var(--accent); border-radius: 2px; outline-offset: 1px; }
.sub-select--set   { opacity: 1; }

.sub-edit {
  flex: 1;
  display: flex;
  align-items: center;
  min-width: 0;
}
.sub-edit-input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  color: var(--muted);
  font-family: inherit;
  font-size: 13px;
  padding: 0;
  outline: none;
}
.sub-edit-input::placeholder { opacity: 0.45; }

.sub-confirm {
  flex: 0 0 auto;
  background: none;
  border: none;
  color: var(--accent);
  font-size: 14px;
  line-height: 1;
  padding: 0 2px 0 4px;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.12s;
}
.sub-confirm:hover { opacity: 1; }

.sub-add {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 14px;
  line-height: 1;
  padding: 2px 4px 4px;
  cursor: pointer;
  opacity: 0.45;
  transition: opacity 0.12s, color 0.12s;
}
.sub-add:hover { color: var(--accent); opacity: 1; }

.sub-gear {
  background: none;
  border: none;
  color: var(--muted);
  line-height: 1;
  padding: 2px 2px 4px 4px;
  cursor: pointer;
  opacity: 0.3;
  transition: opacity 0.12s, color 0.12s;
}
.sub-gear:hover { color: var(--accent); opacity: 0.8; }
</style>
