import { ref, onBeforeUnmount, watch, type Ref } from 'vue'
import type { Card } from '../types'

const ESTIMATED_CARD_HEIGHT = 1400

export function useCardVisibility(cards: Ref<Card[]>) {
  const visible = ref<Record<string, boolean>>({})
  const heights = ref<Record<string, number>>({})
  const sentinels: Record<string, HTMLElement> = {}
  const resizeObservers: Record<string, ResizeObserver> = {}
  let intersectionObserver: IntersectionObserver | null = null

  function setSentinel(id: string, el: HTMLElement | null) {
    if (!el) return
    sentinels[id] = el

    if (intersectionObserver) intersectionObserver.observe(el)

    if (!resizeObservers[id]) {
      const ro = new ResizeObserver(entries => {
        const h = entries[0]?.contentRect.height
        if (h && h > 0) heights.value[id] = h
      })
      ro.observe(el)
      resizeObservers[id] = ro
    }
  }

  function setupIntersection() {
    intersectionObserver?.disconnect()
    intersectionObserver = new IntersectionObserver(
      entries => {
        for (const entry of entries) {
          const id = (entry.target as HTMLElement).dataset.cardId
          if (id) visible.value[id] = entry.isIntersecting
        }
      },
      { rootMargin: '600px 0px' }
    )
    for (const el of Object.values(sentinels)) {
      intersectionObserver.observe(el)
    }
  }

  onBeforeUnmount(() => {
    intersectionObserver?.disconnect()
    for (const ro of Object.values(resizeObservers)) ro.disconnect()
  })

  watch(cards, () => {
    for (const card of cards.value) {
      if (!(card.id in visible.value)) visible.value[card.id] = false
    }
    setupIntersection()
  }, { immediate: true })

  function placeholderStyle(id: string): Record<string, string> {
    const h = heights.value[id] ?? ESTIMATED_CARD_HEIGHT
    return { minHeight: `${h}px` }
  }

  return { visible, placeholderStyle, setSentinel }
}
