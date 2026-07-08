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
  // Also check the boundary itself — handles cases where the panel element
  // is its own scroll container (e.g. sv-backdrop in SuggestionViewer).
  if (boundary.scrollHeight > boundary.clientHeight) return boundary
  return null
}

export function useScrollGuard() {
  function onWheel(e: WheelEvent) {
    const panel = findFloatPanel(e.target as Element)
    if (!panel) return

    const scrollable = findScrollable(e.target as Element, panel)
    if (!scrollable) {
      // Nothing to scroll inside the panel — block so it doesn't reach the body.
      e.preventDefault()
      return
    }

    // Scrollable element found: let native scroll handle it (preserves trackpad
    // momentum/inertia). Only block at the edges so overscroll doesn't chain to body.
    const atTop    = scrollable.scrollTop <= 0 && e.deltaY < 0
    const atBottom = scrollable.scrollTop + scrollable.clientHeight >= scrollable.scrollHeight - 1 && e.deltaY > 0
    if (atTop || atBottom) e.preventDefault()
  }

  onMounted(() => document.addEventListener('wheel', onWheel, { passive: false }))
  onUnmounted(() => document.removeEventListener('wheel', onWheel))
}
