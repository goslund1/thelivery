<template>
  <!-- No car selected yet -->
  <div v-if="!props.carId" class="lp-empty">select a car first</div>

  <!-- Livery set — chip display -->
  <template v-else-if="livery && !creating">
    <span class="lp-chip">
      <span class="lp-serial">{{ livery.serial }}</span>
      <span class="lp-name">{{ livery.name }}</span>
      <button class="lp-clear" @click="clear" title="Clear livery">×</button>
    </span>
  </template>

  <!-- Unset — list existing + new button -->
  <template v-else-if="!creating">
    <div class="lp-wrap">
      <div v-if="loading" class="lp-loading">…</div>
      <template v-else>
        <button
          v-for="l in liveries"
          :key="l.id"
          class="lp-existing-btn"
          @click="select(l.id)"
        >
          <span class="lp-existing-serial">{{ l.serial }}</span>
          {{ l.name }}
        </button>
        <button class="lp-new-btn" @click="startCreate">+ New livery</button>
      </template>
    </div>
  </template>

  <!-- Creating new livery -->
  <template v-else>
    <div class="lp-create-form">
      <div class="lp-factory-toggle">
        <button
          class="lp-toggle-btn"
          :class="{ 'lp-toggle-btn--active': !newIsFactory }"
          @click="newIsFactory = false"
        >Custom</button>
        <button
          class="lp-toggle-btn"
          :class="{ 'lp-toggle-btn--active': newIsFactory }"
          @click="newIsFactory = true"
        >Factory</button>
      </div>
      <input
        ref="nameInputRef"
        class="lp-name-input"
        v-model="newName"
        :placeholder="newIsFactory ? 'Factory color name…' : 'Livery name…'"
        @keydown.enter.prevent="save"
        @keydown.escape.prevent="cancelCreate"
      />
      <div class="lp-create-actions">
        <button class="lp-save-btn" :disabled="!newName.trim() || saving" @click="save">
          {{ saving ? '…' : 'Save' }}
        </button>
        <button class="lp-cancel-btn" @click="cancelCreate">Cancel</button>
      </div>
      <span v-if="saveError" class="lp-error">{{ saveError }}</span>
    </div>
  </template>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useLiveriesStore } from '../stores/liveries'
import type { Livery } from '../types'

const props = defineProps<{
  carId?: string | null
  liveryId?: number | null
}>()
const emit = defineEmits<{ 'update:liveryId': [id: number | null] }>()

const store = useLiveriesStore()

const liveries = ref<Livery[]>([])
const loading = ref(false)
const creating = ref(false)
const saving = ref(false)
const saveError = ref('')
const newName = ref('')
const newIsFactory = ref(false)
const nameInputRef = ref<HTMLInputElement>()

const livery = computed<Livery | undefined>(() =>
  props.liveryId ? store.get(props.liveryId) : undefined
)

watch(() => props.carId, async (id) => {
  liveries.value = []
  creating.value = false
  if (!id) return
  loading.value = true
  liveries.value = await store.loadForCar(id)
  loading.value = false
}, { immediate: true })

function select(id: number) {
  emit('update:liveryId', id)
}

function clear() {
  emit('update:liveryId', null)
}

async function startCreate() {
  newName.value = ''
  newIsFactory.value = false
  saveError.value = ''
  creating.value = true
  await nextTick()
  nameInputRef.value?.focus()
}

function cancelCreate() {
  creating.value = false
}

async function save() {
  if (!props.carId || !newName.value.trim() || saving.value) return
  saving.value = true
  saveError.value = ''
  const result = await store.create({
    carId: props.carId,
    name: newName.value.trim(),
    isFactory: newIsFactory.value,
  })
  saving.value = false
  if (!result) {
    saveError.value = 'Failed to save — try again'
    return
  }
  liveries.value = await store.loadForCar(props.carId)
  creating.value = false
  emit('update:liveryId', result.id)
}
</script>

<style scoped>
.lp-empty {
  font: 11px/1 'Oswald', sans-serif;
  color: var(--text-muted, #666);
  font-style: italic;
}

.lp-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px 3px 4px;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #444);
  background: color-mix(in srgb, var(--panel-well, #1a1a1a) 60%, transparent);
}
.lp-serial {
  font: 700 9px/1 'Oswald', sans-serif;
  letter-spacing: 0.05em;
  color: var(--accent, #c9aa71);
  opacity: 0.8;
}
.lp-name {
  font: 12px/1.2 'Oswald', sans-serif;
  color: var(--text-primary, #e0e0e0);
}
.lp-clear {
  font: 14px/1 monospace;
  background: none;
  border: none;
  color: var(--text-muted, #888);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
}
.lp-clear:hover { color: var(--text-primary, #e0e0e0); }

.lp-wrap {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
}
.lp-loading {
  font: 11px/1 'Oswald', sans-serif;
  color: var(--text-muted, #666);
}

.lp-existing-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font: 11px/1 'Oswald', sans-serif;
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #555);
  background: transparent;
  color: var(--text-secondary, #ccc);
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.lp-existing-btn:hover {
  border-color: var(--accent, #c9aa71);
  color: var(--text-primary, #e0e0e0);
}
.lp-existing-serial {
  font-size: 9px;
  color: var(--accent, #c9aa71);
  opacity: 0.7;
}

.lp-new-btn {
  font: 11px/1 'Oswald', sans-serif;
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px dashed var(--muted-light, #555);
  background: transparent;
  color: var(--text-muted, #888);
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.lp-new-btn:hover {
  border-color: var(--accent, #c9aa71);
  color: var(--accent, #c9aa71);
}

.lp-create-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.lp-factory-toggle {
  display: flex;
  gap: 0;
}
.lp-toggle-btn {
  font: 11px/1 'Oswald', sans-serif;
  padding: 3px 10px;
  border: 1px solid var(--muted-light, #555);
  background: transparent;
  color: var(--text-muted, #888);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.lp-toggle-btn:first-child { border-radius: 4px 0 0 4px; }
.lp-toggle-btn:last-child  { border-radius: 0 4px 4px 0; border-left: none; }
.lp-toggle-btn--active {
  background: var(--accent, #c9aa71);
  color: #000;
  border-color: var(--accent, #c9aa71);
}

.lp-name-input {
  font: 12px/1 'Oswald', sans-serif;
  padding: 4px 7px;
  border-radius: 4px;
  border: 1px solid var(--accent, #c9aa71);
  background: color-mix(in srgb, var(--panel-well, #1a1a1a) 80%, transparent);
  color: var(--text-primary, #e0e0e0);
  outline: none;
}
.lp-name-input::placeholder { color: var(--text-muted, #666); }

.lp-create-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}
.lp-save-btn {
  font: 11px/1 'Oswald', sans-serif;
  padding: 4px 12px;
  border-radius: 4px;
  border: 1px solid var(--accent, #c9aa71);
  background: var(--accent, #c9aa71);
  color: #000;
  cursor: pointer;
}
.lp-save-btn:disabled { opacity: 0.4; cursor: default; }
.lp-cancel-btn {
  font: 11px/1 'Oswald', sans-serif;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--muted-light, #555);
  background: transparent;
  color: var(--text-muted, #888);
  cursor: pointer;
}
.lp-cancel-btn:hover { color: var(--text-primary, #e0e0e0); }

.lp-error {
  font: 11px/1 'Oswald', sans-serif;
  color: #e03030;
}
</style>
