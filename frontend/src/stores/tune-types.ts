import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { TuneType } from '../types'
import { useAuthStore } from './auth'

export const useTuneTypesStore = defineStore('tuneTypes', () => {
  const all = ref<TuneType[]>([])
  const loaded = ref(false)

  async function load() {
    if (loaded.value) return
    const res = await fetch('/api/tune-types')
    if (!res.ok) return
    all.value = await res.json()
    loaded.value = true
  }

  function byId(id: number): TuneType | undefined {
    return all.value.find(t => t.id === id)
  }

  async function create(name: string, sortOrder?: number): Promise<TuneType | null> {
    const auth = useAuthStore()
    const res = await fetch('/api/tune-types', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${auth.token}` },
      body: JSON.stringify({ name, sortOrder }),
    })
    if (!res.ok) return null
    const created = await res.json() as { id: number; name: string }
    const entry: TuneType = { id: created.id, name: created.name, sortOrder: sortOrder ?? 0 }
    all.value = [...all.value, entry].sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name))
    return entry
  }

  return { all, loaded, load, byId, create }
})
