import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Livery } from '../types'
import { useAuthStore } from './auth'

export const useLiveriesStore = defineStore('liveries', () => {
  // Keyed by carId — populated on demand when a car's liveries are first needed.
  const byCarId = ref<Map<string, Livery[]>>(new Map())
  // Flat lookup by livery id.
  const byId = ref<Map<number, Livery>>(new Map())
  let loadedAll = false

  // Populate byId with every livery. The color filter matches cards through
  // liveryId → livery color, so it needs the full lookup even for cars whose
  // liveries were never loaded on demand.
  async function loadAll(): Promise<void> {
    if (loadedAll) return
    const res = await fetch('/api/liveries')
    if (!res.ok) return
    const rows = await res.json() as Livery[]
    for (const l of rows) byId.value.set(l.id, l)
    loadedAll = true
  }

  async function loadForCar(carId: string): Promise<Livery[]> {
    if (byCarId.value.has(carId)) return byCarId.value.get(carId)!
    const res = await fetch(`/api/liveries?carId=${encodeURIComponent(carId)}`)
    if (!res.ok) return []
    const rows = await res.json() as Livery[]
    byCarId.value = new Map(byCarId.value).set(carId, rows)
    for (const l of rows) byId.value.set(l.id, l)
    return rows
  }

  function get(id: number): Livery | undefined {
    return byId.value.get(id)
  }

  async function create(payload: {
    carId: string
    name: string
    isFactory?: boolean
    carColorId?: number
    shareCode?: string
    colorPrimary?: string
    colorSecondary?: string
  }): Promise<{ id: number; serial: string }> {
    const auth = useAuthStore()
    const res = await fetch('/api/liveries', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${auth.token}` },
      body: JSON.stringify(payload),
    })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      throw new Error(text || `HTTP ${res.status}`)
    }
    const created = await res.json() as { id: number; serial: string }
    // Invalidate the cache for this car so next loadForCar re-fetches.
    const updated = new Map(byCarId.value)
    updated.delete(payload.carId)
    byCarId.value = updated
    return created
  }

  async function update(id: number, payload: Partial<Omit<Livery, 'id' | 'carId' | 'serial' | 'createdAt'>>): Promise<boolean> {
    const auth = useAuthStore()
    const res = await fetch(`/api/liveries/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${auth.token}` },
      body: JSON.stringify(payload),
    })
    if (!res.ok) return false
    const existing = byId.value.get(id)
    if (existing) {
      const merged = { ...existing, ...payload }
      byId.value.set(id, merged)
      const carRows = byCarId.value.get(existing.carId)
      if (carRows) {
        byCarId.value = new Map(byCarId.value).set(
          existing.carId,
          carRows.map(l => l.id === id ? merged : l),
        )
      }
    }
    return true
  }

  async function remove(id: number): Promise<boolean> {
    const auth = useAuthStore()
    const res = await fetch(`/api/liveries/${id}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${auth.token}` },
    })
    if (!res.ok) return false
    const existing = byId.value.get(id)
    byId.value.delete(id)
    if (existing) {
      const carRows = byCarId.value.get(existing.carId)
      if (carRows) {
        byCarId.value = new Map(byCarId.value).set(
          existing.carId,
          carRows.filter(l => l.id !== id),
        )
      }
    }
    return true
  }

  return { byCarId, byId, loadAll, loadForCar, get, create, update, remove }
})
