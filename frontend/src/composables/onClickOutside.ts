import { onBeforeUnmount, onMounted, type Ref } from 'vue'

// Calls `handler` when a pointerdown happens outside the given element.
// Used to dismiss flyouts/menus. Mirrors the original app's document-level
// "close all flyouts on outside click" behavior.
export function onClickOutside(elRef: Ref<HTMLElement | null>, handler: () => void) {
  function onDown(e: MouseEvent) {
    const el = elRef.value
    if (el && !el.contains(e.target as Node)) handler()
  }
  onMounted(() => document.addEventListener('pointerdown', onDown, true))
  onBeforeUnmount(() => document.removeEventListener('pointerdown', onDown, true))
}
