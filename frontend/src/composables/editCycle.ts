import { computed, nextTick, ref, watch } from 'vue'
import { useUiStore } from '../stores/ui'
import { refreshTip } from './tooltip'

// "Jump to next edit": cycles through every individually-typed field across all
// dirty cards. Module-level state so the desktop dock (EditBar) and the phone
// side drawer (SideBug) share one position — <script setup> state is per instance.
const cycleIndex = ref(0)
const lastJumpIndex = ref(-1)
let _highlightTimer: ReturnType<typeof setTimeout> | null = null
let _watching = false

export function useEditCycle() {
  const ui = useUiStore()
  const editCount = computed(() => ui.editCount)

  if (!_watching) {
    _watching = true
    // When the user manually focuses a known edit field, sync the pill position.
    watch(() => ui.currentEditIndex, (idx) => {
      if (idx >= 0) lastJumpIndex.value = idx
    })
    // Reset counters when saved cards remove entries from the edit list.
    watch(editCount, (next, prev) => {
      if (next < (prev ?? 0)) {
        cycleIndex.value = 0
        lastJumpIndex.value = -1
      }
    })
  }

  async function onCycleDirty() {
    const count = ui.editCount
    if (!count) return

    const idx = cycleIndex.value % count
    lastJumpIndex.value = idx
    cycleIndex.value = (cycleIndex.value + 1) % count

    const entry = ui.getEditAt(idx)
    if (!entry) return
    const target = entry.el as HTMLElement

    // Open a collapsed <details> section before measuring position
    const details = target.closest('details') as HTMLDetailsElement | null
    if (details && !details.open) {
      details.querySelector('summary')?.click()
      await nextTick()
    }

    // Center in viewport — more predictable than scrollIntoView on a tall page
    const rect = target.getBoundingClientRect()
    const midpoint = rect.top + rect.height / 2
    const delta = midpoint - window.innerHeight / 2
    if (Math.abs(delta) > 4) {
      window.scrollBy({ top: delta, behavior: 'smooth' })
    }

    // Focus and restore cursor
    if (typeof target.focus === 'function') {
      target.focus({ preventScroll: true })
      if (entry.range) {
        try {
          const sel = window.getSelection()
          sel?.removeAllRanges()
          sel?.addRange(entry.range)
        } catch {
          // Range stale — leave cursor where focus placed it
        }
      }
    }

    // Pulse gold highlight ring
    if (_highlightTimer !== null) clearTimeout(_highlightTimer)
    target.classList.remove('edit-jump-target')
    void target.offsetWidth // force reflow to restart animation
    target.classList.add('edit-jump-target')
    _highlightTimer = setTimeout(() => {
      target.classList.remove('edit-jump-target')
      _highlightTimer = null
    }, 1600)

    refreshTip('Jump to next edit')
  }

  const pillLabel = computed(() => {
    if (lastJumpIndex.value < 0) return `${editCount.value} edit${editCount.value === 1 ? '' : 's'}`
    return `Edit ${lastJumpIndex.value + 1} / ${editCount.value}`
  })

  return { editCount, onCycleDirty, pillLabel }
}
