// Custom tooltip — one shared element used by every trigger (favorite star,
// "Build It", side-bug buttons, exit-edit). It drawer-slides open: width
// animates 0 -> natural content width, snapping shut first so switching between
// triggers restarts the reveal cleanly. Position is computed once on open and
// the tip closes on scroll (it can't track a moved target). Faithful port of the
// original app's showCustomTip/hideCustomTip.
import type { Directive } from 'vue'

let tipEl: HTMLElement | null = null
let innerEl: HTMLElement | null = null

export function registerTip(tip: HTMLElement, inner: HTMLElement) {
  tipEl = tip
  innerEl = inner
}

export function showTip(target: HTMLElement, text: string) {
  if (!tipEl || !innerEl) return
  const r = target.getBoundingClientRect()
  innerEl.textContent = text

  // Snap shut instantly first — fresh start even if the mouse jumped here
  // straight from another trigger.
  tipEl.style.transition = 'none'
  tipEl.style.width = '0px'
  tipEl.style.opacity = '0'
  tipEl.style.top = `${r.top + r.height / 2 - 14}px`
  tipEl.style.right = `${window.innerWidth - r.left + 10}px`

  requestAnimationFrame(() => {
    const naturalWidth = (innerEl as HTMLElement).scrollWidth + 22 // + L/R padding
    requestAnimationFrame(() => {
      if (!tipEl) return
      tipEl.style.transition = 'width .35s cubic-bezier(.16,1,.3,1), opacity .15s ease'
      tipEl.style.width = `${naturalWidth}px`
      tipEl.style.opacity = '1'
    })
  })
}

export function hideTip() {
  if (!tipEl) return
  tipEl.style.transition = 'width .25s cubic-bezier(.4,0,1,1), opacity .15s ease'
  tipEl.style.width = '0px'
  tipEl.style.opacity = '0'
}

// The tip is positioned at open time only; if the page scrolls it would strand
// itself, so just close it.
if (typeof window !== 'undefined') {
  window.addEventListener('scroll', () => hideTip(), { passive: true })
}

// v-tip="'text'" or v-tip="() => dynamicText" — the function form is evaluated
// on each hover so it reflects current state (e.g. favorited, theme, open).
type TipValue = string | (() => string)
interface TipEl extends HTMLElement {
  _tipRaw?: TipValue
  _tipEnter?: () => void
  _tipLeave?: () => void
}

export const vTip: Directive<TipEl, TipValue> = {
  mounted(el, binding) {
    el._tipRaw = binding.value
    el._tipEnter = () => {
      const v = el._tipRaw
      showTip(el, typeof v === 'function' ? v() : (v ?? ''))
    }
    el._tipLeave = () => hideTip()
    el.addEventListener('mouseenter', el._tipEnter)
    el.addEventListener('mouseleave', el._tipLeave)
  },
  updated(el, binding) {
    el._tipRaw = binding.value
  },
  beforeUnmount(el) {
    if (el._tipEnter) el.removeEventListener('mouseenter', el._tipEnter)
    if (el._tipLeave) el.removeEventListener('mouseleave', el._tipLeave)
    hideTip()
  },
}
