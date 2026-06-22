<script setup lang="ts">
import { ref, watch, nextTick, toRef } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useSlideshow } from '../composables/useSlideshow'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const images = toRef(props.card, 'images')
const stageRef = ref<HTMLElement | null>(null)
const barRef = ref<HTMLElement | null>(null)
const toggleRef = ref<HTMLElement | null>(null)
const { ordered, index, toggleIcon, toggleLabel, toggle, onThumb } = useSlideshow(
  images,
  stageRef,
  barRef,
  toggleRef,
)

const thumbsRef = ref<HTMLElement | null>(null)
const canLeft = ref(false)
const canRight = ref(false)

function updateArrows() {
  const t = thumbsRef.value
  if (!t) return
  const maxScroll = t.scrollWidth - t.clientWidth
  canLeft.value = t.scrollLeft > 4
  canRight.value = t.scrollLeft < maxScroll - 4
}

// Keep the active thumbnail visible as the slide changes.
// We scroll only the .thumbs container horizontally — never the page.
// scrollIntoView with block:'nearest' still propagates up the scroll
// chain and moves the whole page vertically, so we use scrollLeft
// arithmetic instead. The container already has scroll-behavior:smooth
// in CSS so the transition is handled there.
watch(index, async () => {
  await nextTick()
  const t = thumbsRef.value
  const active = t?.querySelector<HTMLElement>('.thumb.active')
  if (t && active) {
    const railLeft = t.scrollLeft
    const railRight = railLeft + t.clientWidth
    const thumbLeft = active.offsetLeft
    const thumbRight = thumbLeft + active.offsetWidth
    if (thumbLeft < railLeft) {
      t.scrollLeft = thumbLeft - 10
    } else if (thumbRight > railRight) {
      t.scrollLeft = thumbRight - t.clientWidth + 10
    }
  }
  updateArrows()
})

// Drag-to-reorder (only meaningful in edit mode; thumbs are draggable).
let dragFrom = -1
function onDragStart(i: number) {
  dragFrom = i
}
function onDrop(i: number) {
  if (dragFrom >= 0 && dragFrom !== i) {
    store.reorderImages(props.card.id, dragFrom, i)
    ui.markCardDirty(props.card.id)
  }
  dragFrom = -1
}
function setLead(imageId: string) {
  store.setLeadImage(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}
</script>

<template>
  <div class="stage" ref="stageRef" :data-group="card.id">
    <div class="progress" ref="barRef"></div>
    <img
      v-for="(img, i) in ordered"
      :key="img.id"
      :src="img.path"
      :class="{ active: i === index }"
      :data-index="i"
      @click="toggle"
    />
    <div class="stage-controls">
      <button class="play-toggle" ref="toggleRef" title="Pause or resume the slideshow" @click="toggle">
        <span class="icon" v-if="toggleIcon">{{ toggleIcon }}</span>
        <span class="label">{{ toggleLabel }}</span>
      </button>
    </div>
  </div>

  <div class="thumb-rail">
    <div class="edge-arrow edge-arrow-left" :class="{ visible: canLeft }"><div class="tri"></div></div>
    <div class="thumbs" ref="thumbsRef" @scroll="updateArrows">
      <div
        v-for="(img, i) in ordered"
        :key="img.id"
        class="thumb"
        :class="{ active: i === index }"
        :data-index="i"
        draggable="true"
        title="View this photo"
        @click="onThumb(i)"
        @dragstart="onDragStart(i)"
        @dragover.prevent
        @drop.prevent="onDrop(i)"
      >
        <img :src="img.path" />
        <button
          class="lead-star"
          type="button"
          title="Set as lead image"
          aria-label="Set as lead image"
          @click.stop="setLead(img.id)"
        >★</button>
      </div>
    </div>
    <div class="edge-arrow edge-arrow-right" :class="{ visible: canRight }"><div class="tri"></div></div>
  </div>
</template>
