<script setup lang="ts">
import { ref, watch } from 'vue'
import { useUiStore } from '../stores/ui'

const ui = useUiStore()

// Progressive load: show the display src (JPEG) immediately, swap to original once loaded.
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
      // Only swap if this lightbox is still open for the same image
      if (ui.lightboxSrc === display) {
        shownSrc.value = original
        isUpgraded.value = true
      }
    }
    preload.src = original
  }
})

function downloadFilename(src: string) {
  return src.split('/').pop() ?? 'download.jpg'
}
</script>

<template>
  <div class="lightbox" :class="{ open: !!ui.lightboxSrc }" @click.self="ui.closeLightbox()">
    <button class="lightbox-close" aria-label="Close" @click="ui.closeLightbox()">×</button>

    <img v-if="shownSrc" :src="shownSrc" alt="" :class="{ upgraded: isUpgraded }" />

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
/* Subtle dissolve when the full-res swap happens */
.lightbox img.upgraded {
  animation: lb-upgrade 0.4s ease;
}
@keyframes lb-upgrade {
  from { opacity: 0.7; }
  to   { opacity: 1; }
}
</style>
