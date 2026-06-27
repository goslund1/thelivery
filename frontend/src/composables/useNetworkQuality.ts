import { ref, onMounted, onBeforeUnmount } from 'vue'
import type { CardImage } from '../types'

// navigator.connection is a Chrome/Android API — not available in Safari.
// We degrade gracefully: unknown network = serve stage quality.
type Quality = 'thumb' | 'stage' | 'full'

function getQuality(): Quality {
  const conn = (navigator as any).connection
  if (!conn) return 'stage' // Safari / unknown — serve stage JPEG
  if (conn.saveData) return 'thumb'
  switch (conn.effectiveType) {
    case 'slow-2g':
    case '2g': return 'thumb'
    case '3g':  return 'stage'
    default:    return 'full'  // 4g or wifi
  }
}

export function useNetworkQuality() {
  const quality = ref<Quality>(getQuality())

  function update() { quality.value = getQuality() }

  onMounted(() => {
    const conn = (navigator as any).connection
    conn?.addEventListener('change', update)
  })
  onBeforeUnmount(() => {
    const conn = (navigator as any).connection
    conn?.removeEventListener('change', update)
  })

  // Returns the best available src for a given image + context.
  function srcFor(img: CardImage, context: 'thumb' | 'stage' | 'lightbox'): string {
    if (context === 'lightbox') return img.path // always full original in lightbox
    if (context === 'thumb') return img.thumbPath ?? img.path
    // stage: respect network quality
    if (quality.value === 'thumb') return img.thumbPath ?? img.path
    if (quality.value === 'full')  return img.path
    return img.stagePath ?? img.path
  }

  return { quality, srcFor }
}
