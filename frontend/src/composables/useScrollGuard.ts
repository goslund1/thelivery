import { onMounted, onUnmounted } from 'vue'

const PANEL_RE = /\b(float_\w+_panel|drawer_\w+_panel)\b/

function findFloatPanel(target: Element): Element | null {
  let el: Element | null = target
  while (el && el !== document.documentElement) {
    if (typeof el.className === 'string' && PANEL_RE.test(el.className)) return el
    el = el.parentElement
  }
  return null
}

function findScrollable(from: Element, boundary: Element): Element | null {
  let el: Element | null = from
  while (el) {
    if (el.scrollHeight > el.clientHeight) return el
    if (el === boundary) break
    el = el.parentElement
  }
  return null
}

export function useScrollGuard() {
  function onWheel(e: WheelEvent) {
    const panel = findFloatPanel(e.target as Element)
    if (!panel) return
    e.preventDefault()
    const scrollable = findScrollable(e.target as Element, panel)
    if (scrollable) scrollable.scrollTop += e.deltaY
  }

  onMounted(() => document.addEventListener('wheel', onWheel, { passive: false }))
  onUnmounted(() => document.removeEventListener('wheel', onWheel))
}
