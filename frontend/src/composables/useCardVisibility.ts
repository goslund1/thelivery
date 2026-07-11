import { ref, onMounted, onBeforeUnmount, watch, type Ref } from 'vue'
import type { Card } from '../types'

export function useCardVisibility(cards: Ref<Card[]>) {
  const visible = ref<Record<string, boolean>>({})
  const sentinels = ref<Record<string, HTMLElement | null>>({})
  let observer: IntersectionObserver | null = null

  function setSentinel(id: string, el: HTMLElement | null) {
    sentinels.value[id] = el
    if (el && observer) observer.observe(el)
  }

  function setup() {
    observer?.disconnect()
    observer = new IntersectionObserver(
      entries => {
        for (const entry of entries) {
          const id = (entry.target as HTMLElement).dataset.cardId
          if (id) visible.value[id] = entry.isIntersecting
        }
      },
      { rootMargin: '600px 0px' }
    )
    for (const el of Object.values(sentinels.value)) {
      if (el) observer.observe(el)
    }
  }

  onMounted(setup)
  onBeforeUnmount(() => observer?.disconnect())

  // Re-run when the card list changes so newly added sentinels get observed
  watch(cards, () => {
    for (const card of cards.value) {
      if (!(card.id in visible.value)) visible.value[card.id] = false
    }
    setup()
  }, { immediate: true })

  return { visible, setSentinel }
}
