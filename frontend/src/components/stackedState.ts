import { ref } from 'vue'
import type { Ref } from 'vue'

// Shared per-card stacked refs. Lives outside <script setup> so it's a true
// module singleton — all concurrent TuningAdjustments instances for the same
// cardId share the exact same Ref<boolean>, keeping them in sync even when the
// virtual-scroll pool assigns two slots to the same card simultaneously.
const _stackedRefs: Record<string, Ref<boolean>> = {}

export function getStackedRef(cardId: string): Ref<boolean> {
  if (!_stackedRefs[cardId]) _stackedRefs[cardId] = ref(false)
  return _stackedRefs[cardId]
}
