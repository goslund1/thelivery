<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useModalStore } from '../stores/modal'

const modal = useModalStore()

const shownSrc = ref<string | null>(null)
const isUpgraded = ref(false)

watch(() => modal.lightboxSrc, (display) => {
  if (!display) { shownSrc.value = null; isUpgraded.value = false; return }
  shownSrc.value = display
  isUpgraded.value = false

  const original = modal.lightboxOriginalSrc
  if (original && original !== display) {
    const preload = new Image()
    preload.onload = () => {
      if (modal.lightboxSrc === display) {
        shownSrc.value = original
        isUpgraded.value = true
      }
    }
    preload.src = original
  }
})

function onKey(e: KeyboardEvent) {
  if (!modal.lightboxSrc) return
  if (e.key === 'ArrowRight') { e.preventDefault(); modal.navigateLightbox(1) }
  if (e.key === 'ArrowLeft')  { e.preventDefault(); modal.navigateLightbox(-1) }
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

const hasMultiple = () => modal.lightboxImages.length > 1

// G6: swipe left/right on touch. A mostly-horizontal move over 40px steps the
// image; the backdrop tap-to-close ignores the click a swipe can leave behind.
let swipeStart: { x: number; y: number } | null = null
let justSwiped = false
function onPointerDown(e: PointerEvent) {
  swipeStart = e.pointerType === 'mouse' ? null : { x: e.clientX, y: e.clientY }
}
function onPointerUp(e: PointerEvent) {
  if (!swipeStart) return
  const dx = e.clientX - swipeStart.x
  const dy = e.clientY - swipeStart.y
  swipeStart = null
  if (Math.abs(dx) > 40 && Math.abs(dx) > Math.abs(dy) && hasMultiple()) {
    modal.navigateLightbox(dx < 0 ? 1 : -1)
    justSwiped = true
    setTimeout(() => { justSwiped = false }, 400)
  }
}
function onBackdropClick() {
  if (justSwiped) return
  modal.closeLightbox()
}

function downloadFilename(src: string) {
  return src.split('/').pop() ?? 'download.jpg'
}
</script>

<template>
  <div class="lightbox" :class="{ open: !!modal.lightboxSrc }" @click.self="onBackdropClick"
    @pointerdown="onPointerDown" @pointerup="onPointerUp" @pointercancel="swipeStart = null">
    <button class="lightbox-close" aria-label="Close" @click="modal.closeLightbox()">×</button>

    <!-- Left ear -->
    <button
      v-if="hasMultiple()"
      class="lightbox-ear lightbox-ear-left"
      aria-label="Previous image"
      @click.stop="modal.navigateLightbox(-1)"
    >‹</button>

    <img v-if="shownSrc" :src="shownSrc" alt="" :class="{ upgraded: isUpgraded }" />

    <!-- Right ear -->
    <button
      v-if="hasMultiple()"
      class="lightbox-ear lightbox-ear-right"
      aria-label="Next image"
      @click.stop="modal.navigateLightbox(1)"
    >›</button>

    <!-- Image counter -->
    <span v-if="hasMultiple()" class="lightbox-counter">
      {{ modal.lightboxIndex + 1 }} / {{ modal.lightboxImages.length }}
    </span>

    <a
      v-if="modal.lightboxOriginalSrc"
      class="lightbox-download"
      :href="modal.lightboxOriginalSrc"
      :download="downloadFilename(modal.lightboxOriginalSrc)"
      title="Download full-resolution original"
      @click.stop
    >↓ Download</a>
  </div>
</template>

<style scoped>
.lightbox-ear {
  position: fixed;
  top: 50%;
  transform: translateY(-50%);
  width: 52px;
  height: 52px;
  background: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 50%;
  color: rgba(255, 255, 255, 0.8);
  font-size: 32px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10002;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
  padding: 0;
}
.lightbox-ear:hover {
  background: rgba(0, 0, 0, 0.85);
  border-color: var(--accent);
  color: var(--accent);
}
.lightbox-ear-left  { left: 20px; }
.lightbox-ear-right { right: 20px; }

.lightbox-counter {
  position: fixed;
  top: 18px;
  left: 50%;
  transform: translateX(-50%);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.1em;
  color: rgba(255, 255, 255, 0.5);
  z-index: 10001;
  pointer-events: none;
}

.lightbox-download {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  text-decoration: none;
  padding: 8px 20px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  z-index: 10001;
}
.lightbox-download:hover {
  background: rgba(0, 0, 0, 0.9);
  border-color: var(--accent);
  color: var(--accent);
}
.lightbox img.upgraded {
  animation: lb-upgrade 0.4s ease;
}
@keyframes lb-upgrade {
  from { opacity: 0.7; }
  to   { opacity: 1; }
}

/* Touch: our pointer events get horizontal swipes; pinch-zoom still works. */
@media (hover: none), (pointer: coarse) {
  .lightbox { touch-action: pinch-zoom; }
}
/* Phones (G6): the arrows sit on the image at this width; move them to a bar
   below it, above the download link. */
@media (max-width: 640px) {
  .lightbox-ear {
    top: auto;
    bottom: calc(72px + env(safe-area-inset-bottom));
    transform: none;
    width: 52px;
    height: 52px;
  }
}
</style>
