import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Car } from '../types'

export const useCarsStore = defineStore('cars', () => {
  const all = ref<Car[]>([])
  const loaded = ref(false)

  async function load() {
    if (loaded.value) return
    const res = await fetch('/api/cars')
    if (!res.ok) return
    all.value = await res.json()
    loaded.value = true
  }

  const fh5 = computed(() => all.value.filter(c => c.game === 'FH5'))
  const fh6 = computed(() => all.value.filter(c => c.game === 'FH6'))

  function byId(id: string): Car | undefined {
    return all.value.find(c => c.id === id)
  }

  function search(game: 'FH5' | 'FH6', query: string): Car[] {
    const q = query.trim().toLowerCase()
    const pool = game === 'FH5' ? fh5.value : fh6.value
    if (!q) return pool.slice(0, 50)
    return pool.filter(c =>
      `${c.make} ${c.model} ${c.year ?? ''}`.toLowerCase().includes(q)
    ).slice(0, 50)
  }

  return { all, loaded, load, fh5, fh6, byId, search }
})
