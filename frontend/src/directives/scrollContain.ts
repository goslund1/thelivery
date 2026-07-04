import type { Directive } from 'vue'

// Prevents wheel events from escaping a floating panel to the page behind,
// whether the panel has scrollable content or not.
function handler(e: WheelEvent) {
  let target = e.target as HTMLElement | null
  const panel = e.currentTarget as HTMLElement

  // Walk up from the event target looking for a scrollable ancestor inside the panel.
  let scrollable: HTMLElement | null = null
  while (target && target !== panel.parentElement) {
    if (target === panel) break
    const oy = getComputedStyle(target).overflowY
    if ((oy === 'auto' || oy === 'scroll') && target.scrollHeight > target.clientHeight) {
      scrollable = target
      break
    }
    target = target.parentElement
  }

  if (!scrollable) {
    e.preventDefault()
    return
  }

  const atTop    = scrollable.scrollTop <= 0 && e.deltaY < 0
  const atBottom = scrollable.scrollTop + scrollable.clientHeight >= scrollable.scrollHeight - 1 && e.deltaY > 0
  if (atTop || atBottom) e.preventDefault()
}

export const vScrollContain: Directive<HTMLElement> = {
  mounted(el) {
    el.addEventListener('wheel', handler as EventListener, { passive: false })
  },
  beforeUnmount(el) {
    el.removeEventListener('wheel', handler as EventListener)
  },
}
