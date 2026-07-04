import { ref } from 'vue'
import type { Ref } from 'vue'

export interface UseDrawerReturn {
  open: Ref<boolean>
  toggle: () => void
  openDrawer: () => void
  closeDrawer: () => void
}

export function useDrawer(initialOpen = false): UseDrawerReturn {
  const open = ref(initialOpen)
  return {
    open,
    toggle:      () => { open.value = !open.value },
    openDrawer:  () => { open.value = true },
    closeDrawer: () => { open.value = false },
  }
}
