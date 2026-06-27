<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useUiStore } from '../stores/ui'

const ui = useUiStore()

const shownSrc = ref<string | null>(null)
const isUpgraded = ref(false)

watch(() => ui.lightboxSrc, (display) => {
  if (!display) { shownSrc.value = null; isUpgraded.value = false; return }
  shownSrc.value = display
  isUpgraded.value = false

  const original = ui.lightboxOriginalSrc
  if (original && original !== display) {
    const preload = new Image()
    preload.onload = () => {
      if (ui.lightboxSrc === display) {
        shownSrc.value = original
        isUpgraded.value = true
      }
    }
    preload.src = original
  }
})

function onKey(e: KeyboardEvent) {
  if (!ui.lightboxSrc) return
  if (e.key === 'ArrowRight') { e.preventDefault(); ui.navigateLightbox(1) }
  if (e.key === 'ArrowLeft')  { e.preventDefault(); ui.navigateLightbox(-1) }
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

const hasMultiple = () => ui.lightboxImages.length > 1

function downloadFilename(src: string) {
  return src.split('/').pop() ?? 'download.jpg'
}
</script>

<template>
  <div class="lightbox" :class="{ open: !!ui.lightboxSrc }" @click.self="ui.closeLightbox()">
    <button class="lightbox-close" aria-label="Close" @click="ui.closeLightbox()">×</button>

    <!-- Left ear -->
    <button
      v-if="hasMultiple()"
      class="lightbox-ear lightbox-ear-left"
      aria-label="Previous image"
      @click.stop="ui.navigateLightbox(-1)"
    >‹</button>

    <img v-if="shownSrc" :src="shownSrc" alt="" :class="{ upgraded: isUpgraded }" />

    <!-- Right ear -->
    <button
      v-if="hasMultiple()"
      class="lightbox-ear lightbox-ear-right"
      aria-label="Next image"
      @click.stop="ui.navigateLightbox(1)"
    >›</button>

    <!-- Image counter -->
    <span v-if="hasMultiple()" class="lightbox-counter">
      {{ ui.lightboxIndex + 1 }} / {{ ui.lightboxImages.length }}
    </span>

    <a
      v-if="ui.lightboxOriginalSrc"
      class="lightbox-download"
      :href="ui.lightboxOriginalSrc"
      :download="downloadFilename(ui.lightboxOriginalSrc)"
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
  border-color: var(--gold);
  color: var(--gold);
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
  color: var(--paper);
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
  border-color: var(--gold);
  color: var(--gold);
}
.lightbox img.upgraded {
  animation: lb-upgrade 0.4s ease;
}
@keyframes lb-upgrade {
  from { opacity: 0.7; }
  to   { opacity: 1; }
}
</style>
