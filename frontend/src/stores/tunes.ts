import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Tune, UpgradeCategory, AdjustmentRow } from '../types'
import { useAuthStore } from './auth'

export const useTunesStore = defineStore('tunes', () => {
  // Keyed by liveryId — populated on demand.
  const byLiveryId = ref<Map<number, Tune[]>>(new Map())
  // Flat lookup by tune id.
  const byId = ref<Map<number, Tune>>(new Map())

  async function loadForLivery(liveryId: number): Promise<Tune[]> {
    if (byLiveryId.value.has(liveryId)) return byLiveryId.value.get(liveryId)!
    const res = await fetch(`/api/tunes?liveryId=${liveryId}`)
    if (!res.ok) return []
    const rows = await res.json() as Tune[]
    byLiveryId.value = new Map(byLiveryId.value).set(liveryId, rows)
    for (const t of rows) byId.value.set(t.id, t)
    return rows
  }

  async function loadForCar(carId: string): Promise<Tune[]> {
    const res = await fetch(`/api/tunes?carId=${encodeURIComponent(carId)}`)
    if (!res.ok) return []
    const rows = await res.json() as Tune[]
    for (const t of rows) {
      byId.value.set(t.id, t)
      const existing = byLiveryId.value.get(t.liveryId) ?? []
      if (!existing.find(e => e.id === t.id)) {
        byLiveryId.value = new Map(byLiveryId.value).set(t.liveryId, [...existing, t])
      }
    }
    return rows
  }

  function get(id: number): Tune | undefined {
    return byId.value.get(id)
  }

  // Helpers to parse the JSON string fields stored in the DB.
  function parsedCoreSpecs(t: Tune): Record<string, string> {
    try { return t.coreSpecs ? JSON.parse(t.coreSpecs) : {} } catch { return {} }
  }
  function parsedUpgrades(t: Tune): UpgradeCategory[] {
    try { return t.upgrades ? JSON.parse(t.upgrades) : [] } catch { return [] }
  }
  function parsedAdjustments(t: Tune): AdjustmentRow[] {
    try { return t.adjustments ? JSON.parse(t.adjustments) : [] } catch { return [] }
  }

  async function create(payload: {
    liveryId: number
    carId: string
    officialName?: string
    typeId?: number
    shareCode?: string
    coreSpecs?: Record<string, string>
    upgrades?: UpgradeCategory[]
    adjustments?: AdjustmentRow[]
  }): Promise<{ id: number; serial: string } | null> {
    const auth = useAuthStore()
    const res = await fetch('/api/tunes', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${auth.token}` },
      body: JSON.stringify(payload),
    })
    if (!res.ok) return null
    const created = await res.json() as { id: number; serial: string }
    // Invalidate livery cache so next load re-fetches.
    const updated = new Map(byLiveryId.value)
    updated.delete(payload.liveryId)
    byLiveryId.value = updated
    return created
  }

  async function update(id: number, payload: {
    officialName?: string
    typeId?: number
    shareCode?: string
    coreSpecs?: Record<string, string>
    upgrades?: UpgradeCategory[]
    adjustments?: AdjustmentRow[]
  }): Promise<boolean> {
    const auth = useAuthStore()
    const res = await fetch(`/api/tunes/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${auth.token}` },
      body: JSON.stringify(payload),
    })
    if (!res.ok) return false
    const existing = byId.value.get(id)
    if (existing) {
      const merged: Tune = {
        ...existing,
        officialName: payload.officialName ?? existing.officialName,
        typeId: payload.typeId ?? existing.typeId,
        shareCode: payload.shareCode ?? existing.shareCode,
        coreSpecs: payload.coreSpecs ? JSON.stringify(payload.coreSpecs) : existing.coreSpecs,
        upgrades: payload.upgrades ? JSON.stringify(payload.upgrades) : existing.upgrades,
        adjustments: payload.adjustments ? JSON.stringify(payload.adjustments) : existing.adjustments,
      }
      byId.value.set(id, merged)
      const livRows = byLiveryId.value.get(existing.liveryId)
      if (livRows) {
        byLiveryId.value = new Map(byLiveryId.value).set(
          existing.liveryId,
          livRows.map(t => t.id === id ? merged : t),
        )
      }
    }
    return true
  }

  async function remove(id: number): Promise<boolean> {
    const auth = useAuthStore()
    const res = await fetch(`/api/tunes/${id}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${auth.token}` },
    })
    if (!res.ok) return false
    const existing = byId.value.get(id)
    byId.value.delete(id)
    if (existing) {
      const livRows = byLiveryId.value.get(existing.liveryId)
      if (livRows) {
        byLiveryId.value = new Map(byLiveryId.value).set(
          existing.liveryId,
          livRows.filter(t => t.id !== id),
        )
      }
    }
    return true
  }

  return { byLiveryId, byId, loadForLivery, loadForCar, get, parsedCoreSpecs, parsedUpgrades, parsedAdjustments, create, update, remove }
})
