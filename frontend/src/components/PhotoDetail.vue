<template>
  <Teleport to="body">
    <div class="pd-overlay" @click.self="emit('close')" @keydown.esc.window="emit('close')">
      <div class="pd-shell">

        <!-- nav arrows -->
        <button v-if="hasPrev" class="pd-nav pd-nav--prev" @click="emit('prev')" title="Previous photo">‹</button>
        <button v-if="hasNext" class="pd-nav pd-nav--next" @click="emit('next')" title="Next photo">›</button>

        <button class="pd-close" @click="emit('close')">×</button>

        <!-- photo -->
        <div class="pd-stage">
          <img :src="image.stagePath ?? image.path" :alt="image.alt ?? ''" class="pd-img" />
        </div>

        <!-- meta panel -->
        <div class="pd-meta">
          <div class="pd-meta-row">
            <span class="pd-label">Alt text</span>
            <input
              class="pd-alt-input"
              :value="image.alt ?? ''"
              placeholder="Describe this photo…"
              @input="onAlt"
            />
          </div>
          <div class="pd-meta-row pd-meta-row--car">
            <span class="pd-label">Car</span>
            <CarPicker
              :car-id="image.carId ?? cardCarId"
              @update:car-id="onCarId"
            />
          </div>
          <div class="pd-meta-footer">
            <span class="pd-hint">Changes save with the card</span>
            <span class="pd-counter">{{ position }} / {{ total }}</span>
          </div>
        </div>

      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { useCarsStore } from '../stores/cars'
import CarPicker from './CarPicker.vue'
import type { CardImage } from '../types'

const props = defineProps<{
  image: CardImage
  cardCarId?: string | null  // card-level default, shown when photo has no override
  position: number           // 1-based index for display
  total: number
  hasPrev: boolean
  hasNext: boolean
}>()

const emit = defineEmits<{
  close: []
  prev: []
  next: []
  'update:alt': [imageId: string, alt: string]
  'update:carId': [imageId: string, carId: string | null]
}>()

useCarsStore().load()

function onAlt(e: Event) {
  emit('update:alt', props.image.id, (e.target as HTMLInputElement).value)
}

function onCarId(carId: string | null) {
  emit('update:carId', props.image.id, carId)
}
</script>

<style scoped>
.pd-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.82);
  z-index: 500;
  display: flex;
  align-items: center;
  justify-content: center;
}

.pd-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  width: min(92vw, 860px);
  max-height: 92vh;
  border-radius: 8px;
  overflow: hidden;
  background: var(--panel-bg, #1a1a1a);
  border: 1px solid var(--panel-edge, #333);
  box-shadow: 0 12px 48px rgba(0,0,0,0.7);
}

/* navigation arrows */
.pd-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 10;
  background: rgba(0,0,0,0.5);
  border: none;
  color: #fff;
  font-size: 28px;
  line-height: 1;
  padding: 12px 10px;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.15s;
}
.pd-nav:hover { background: rgba(0,0,0,0.75); }
.pd-nav--prev { left: 8px; }
.pd-nav--next { right: 8px; }

.pd-close {
  position: absolute;
  top: 8px;
  right: 10px;
  z-index: 10;
  background: rgba(0,0,0,0.4);
  border: none;
  color: #ccc;
  font-size: 18px;
  line-height: 1;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
}
.pd-close:hover { color: #fff; }

/* photo */
.pd-stage {
  flex: 1;
  min-height: 0;
  background: #000;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.pd-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  display: block;
}

/* meta panel */
.pd-meta {
  flex-shrink: 0;
  padding: 12px 16px 10px;
  border-top: 1px solid var(--panel-edge, #333);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.pd-meta-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.pd-label {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted, #888);
  min-width: 52px;
  flex-shrink: 0;
}

.pd-alt-input {
  flex: 1;
  font: 12px/1 'Oswald', sans-serif;
  padding: 4px 6px;
  border-radius: 4px;
  border: 1px solid var(--steel-light, #444);
  background: color-mix(in srgb, var(--panel-well, #111) 80%, transparent);
  color: var(--text-primary, #e0e0e0);
  outline: none;
}
.pd-alt-input:focus { border-color: var(--accent, #c9aa71); }
.pd-alt-input::placeholder { color: var(--text-muted, #555); }

.pd-meta-row--car { align-items: flex-start; }

.pd-meta-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 2px;
}
.pd-hint {
  font: 10px/1 'Oswald', sans-serif;
  color: var(--text-muted, #666);
  font-style: italic;
}
.pd-counter {
  font: 11px/1 'Oswald', sans-serif;
  color: var(--text-muted, #666);
}
</style>
