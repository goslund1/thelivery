import { ref } from 'vue'
import { defineStore } from 'pinia'

export type ToastItemStatus = 'pending' | 'processing' | 'done' | 'error'

export interface ToastItem {
  id: string
  text: string
  status: ToastItemStatus
  detail?: string
}

export interface Toast {
  id: string
  title: string
  items: ToastItem[]
  fadingOut: boolean
}

let _seq = 0
function uid() { return `t${++_seq}` }

export const useToastsStore = defineStore('toasts', () => {
  const toasts = ref<Toast[]>([])

  function push(title: string, items: Omit<ToastItem, 'id'>[]): string {
    const id = uid()
    toasts.value.push({
      id,
      title,
      items: items.map(i => ({ ...i, id: uid() })),
      fadingOut: false,
    })
    return id
  }

  function addItem(toastId: string, item: Omit<ToastItem, 'id'>): string {
    const t = toasts.value.find(t => t.id === toastId)
    if (!t) return ''
    const id = uid()
    t.items.push({ ...item, id })
    return id
  }

  function updateItem(toastId: string, itemId: string, patch: Partial<ToastItem>) {
    const t = toasts.value.find(t => t.id === toastId)
    if (!t) return
    const item = t.items.find(i => i.id === itemId)
    if (item) Object.assign(item, patch)
    tryFade(toastId)
  }

  function tryFade(toastId: string) {
    const t = toasts.value.find(t => t.id === toastId)
    if (!t || t.fadingOut) return
    const allSettled = t.items.every(i => i.status === 'done' || i.status === 'error')
    if (!allSettled) return
    // Any error → stay open; user must dismiss manually to read it.
    const hasError = t.items.some(i => i.status === 'error')
    if (hasError) return
    setTimeout(() => {
      const t2 = toasts.value.find(t => t.id === toastId)
      if (t2) t2.fadingOut = true
      setTimeout(() => {
        toasts.value = toasts.value.filter(t => t.id !== toastId)
      }, 500)
    }, 2000)
  }

  function dismiss(toastId: string) {
    const t = toasts.value.find(t => t.id === toastId)
    if (t) t.fadingOut = true
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== toastId)
    }, 500)
  }

  return { toasts, push, addItem, updateItem, tryFade, dismiss }
})
