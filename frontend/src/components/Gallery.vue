<script setup lang="ts">
import { ref, computed, watch, nextTick, toRef } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useSlideshow } from '../composables/useSlideshow'
import { useNetworkQuality } from '../composables/useNetworkQuality'
import { api } from '../api'

const props = defineProps<{ card: Card }>()
const store = useCardsStore()
const ui = useUiStore()

const images = toRef(props.card, 'images')

// Full pool sorted by order — shown in edit mode thumb rail.
const poolSorted = computed(() => [...images.value].sort((a, b) => a.order - b.order))

const stageRef = ref<HTMLElement | null>(null)
const barRef = ref<HTMLElement | null>(null)
const toggleRef = ref<HTMLElement | null>(null)

const { srcFor } = useNetworkQuality()

// useSlideshow internally filters to included-only.
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

watch(index, async () => {
  await nextTick()
  const t = thumbsRef.value
  const active = t?.querySelector<HTMLElement>('.thumb.active')
  if (t && active) {
    const railLeft = t.scrollLeft
    const railRight = railLeft + t.clientWidth
    const thumbLeft = active.offsetLeft
    const thumbRight = thumbLeft + active.offsetWidth
    if (thumbLeft < railLeft) t.scrollLeft = thumbLeft - 10
    else if (thumbRight > railRight) t.scrollLeft = thumbRight - t.clientWidth + 10
  }
  updateArrows()
})

let dragFrom = -1
function onDragStart(i: number) { dragFrom = i }
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
function toggleIncluded(imageId: string) {
  store.toggleImageIncluded(props.card.id, imageId)
  ui.markCardDirty(props.card.id)
}

// + button: hidden file input trigger
const addInputRef = ref<HTMLInputElement | null>(null)
const uploadProgress = ref<{ done: number; total: number } | null>(null)
const uploading = computed(() => uploadProgress.value !== null)

async function onAddFile(e: Event) {
  const files = Array.from((e.target as HTMLInputElement).files ?? [])
  if (!files.length) return
  uploadProgress.value = { done: 0, total: files.length }
  try {
    await Promise.all(
      files.map(async (file) => {
        const { path, thumbPath, stagePath } = await api.uploadImage(file, {
          name: props.card.name,
          subtitle: props.card.subtitle,
          collections: props.card.collections,
        })
        store.addImageToPool(props.card.id, path, thumbPath, stagePath)
        ui.markCardDirty(props.card.id)
        uploadProgress.value!.done++
      }),
    )
  } finally {
    uploadProgress.value = null
    if (addInputRef.value) addInputRef.value.value = ''
  }
}
</script>

<template>
  <div class="stage" ref="stageRef" :data-group="card.id">
    <div class="progress" ref="barRef"></div>
    <img
      v-for="(img, i) in ordered"
      :key="img.id"
      :src="srcFor(img, 'stage')"
      :class="{ active: i === index }"
      :data-index="i"
      @click="toggle"
    />
    <div class="stage-controls">
      <button class="play-toggle" ref="toggleRef" title="Pause or resume the slideshow" @click="toggle">
        <span class="icon" v-if="toggleIcon">{{ toggleIcon }}</span>
        <span class="label">{{ toggleLabel }}</span>
      </button>
      <button
        v-if="ordered[index]"
        class="stage-expand"
        title="View full resolution"
        @click.stop="ui.openLightbox(srcFor(ordered[index], 'stage'), ordered[index].path)"
      >⤢</button>
    </div>
  </div>

  <div class="thumb-rail">
    <div class="edge-arrow edge-arrow-left" :class="{ visible: canLeft }"><div class="tri"></div></div>
    <div class="thumbs" ref="thumbsRef" @scroll="updateArrows">

      <!-- Edit mode: show full pool with include/exclude toggle -->
      <template v-if="ui.isEditing">
        <div
          v-for="(img, i) in poolSorted"
          :key="img.id"
          class="thumb"
          :class="{ active: ordered[index]?.id === img.id, excluded: img.included === false }"
          draggable="true"
          title="Drag to reorder"
          @click="onThumb(ordered.findIndex(o => o.id === img.id))"
          @dragstart="onDragStart(i)"
          @dragover.prevent
          @drop.prevent="onDrop(i)"
        >
          <img :src="img.thumbPath ?? img.path" />
          <button
            class="lead-star"
            type="button"
            title="Set as lead image"
            aria-label="Set as lead image"
            @click.stop="setLead(img.id)"
          >★</button>
          <button
            class="pool-toggle"
            type="button"
            :title="img.included === false ? 'Add to slideshow' : 'Remove from slideshow'"
            @click.stop="toggleIncluded(img.id)"
          >{{ img.included === false ? '○' : '●' }}</button>
        </div>

        <!-- + button to add a new image to the pool -->
        <div class="thumb thumb-add" :class="{ loading: uploading }" @click="!uploading && addInputRef?.click()">
          <template v-if="!uploadProgress">
            <span class="thumb-add-icon">+</span>
            <span class="thumb-add-label" v-if="poolSorted.length === 0">Add photos</span>
          </template>
          <span class="thumb-add-progress" v-else>{{ uploadProgress.done }}/{{ uploadProgress.total }}</span>
          <input ref="addInputRef" type="file" accept="image/*" multiple style="display:none" @change="onAddFile" />
        </div>
      </template>

      <!-- View mode: only included images -->
      <template v-else>
        <div
          v-for="(img, i) in ordered"
          :key="img.id"
          class="thumb"
          :class="{ active: i === index }"
          :data-index="i"
          title="View this photo"
          @click="onThumb(i)"
        >
          <img :src="img.thumbPath ?? img.path" />
        </div>
      </template>

    </div>
    <div class="edge-arrow edge-arrow-right" :class="{ visible: canRight }"><div class="tri"></div></div>
  </div>
</template>

<style scoped>
.stage-expand {
  position: absolute;
  bottom: 10px;
  right: 10px;
  width: 28px;
  height: 28px;
  background: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  color: rgba(255, 255, 255, 0.7);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  opacity: 0;
  transition: opacity 0.2s ease, color 0.15s ease, border-color 0.15s ease;
}
.stage:hover .stage-expand {
  opacity: 1;
}
.stage-expand:hover {
  color: var(--gold);
  border-color: var(--gold);
}
.thumb.excluded {
  opacity: 0.35;
}
.thumb.excluded img {
  filter: grayscale(60%);
}
.pool-toggle {
  position: absolute;
  bottom: 4px;
  left: 4px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 1px solid rgba(255,255,255,0.5);
  background: rgba(0,0,0,0.55);
  color: #fff;
  font-size: 10px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.thumb-add {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px dashed var(--panel-edge);
  background: transparent;
  cursor: pointer;
  min-width: 60px;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.thumb-add:hover {
  border-color: var(--gold);
  background: var(--gold-tint-04);
}
.thumb-add-icon {
  font-size: 20px;
  color: var(--steel);
  line-height: 1;
}
.thumb-add:hover .thumb-add-icon {
  color: var(--gold);
}
.thumb-add-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--steel);
  margin-top: 4px;
}
.thumb-add:hover .thumb-add-label {
  color: var(--gold);
}
.thumb-add-progress {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--gold);
  letter-spacing: 0.05em;
}
</style>
