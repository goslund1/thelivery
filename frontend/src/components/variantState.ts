import { ref } from 'vue'
import type { Ref } from 'vue'

const _activeVariantIndexRefs: Record<string, Ref<number>> = {}

export function getActiveVariantIndex(cardId: string): Ref<number> {
  if (!_activeVariantIndexRefs[cardId]) _activeVariantIndexRefs[cardId] = ref(0)
  return _activeVariantIndexRefs[cardId]
}
