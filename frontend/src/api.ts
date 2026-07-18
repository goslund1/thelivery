// Thin API client for the Rust backend. Uses relative URLs (dev server proxies
// /api and /uploads to :8787; in prod they are same-origin). Mutating calls send
// the JWT as a Bearer header; reads are public.
import type { Card, UpgradeCategory } from './types'

export class ApiError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new ApiError(res.status, `${res.status} ${res.statusText}: ${await res.text()}`)
  return res.json() as Promise<T>
}

// Bearer header from the stored token (the auth store keeps this in sync).
function authHeaders(): Record<string, string> {
  const t = localStorage.getItem('auth_token')
  return t ? { authorization: `Bearer ${t}` } : {}
}

export const api = {
  login: (username: string, password: string) =>
    fetch('/api/login', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ username, password }),
    }).then(json<{ token: string; username: string; role: 'admin' | 'editor'; mustChangePassword: boolean }>),

  listCards: () => fetch('/api/cards').then(json<Card[]>),

  getCard: (id: string) => fetch(`/api/cards/${id}`).then(json<Card>),

  saveCard: (card: Card) =>
    fetch(`/api/cards/${card.id}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify(card),
    }).then(json<Card>),

  createCard: (card: Card) =>
    fetch('/api/cards', {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify(card),
    }).then(json<Card>),

  deleteCard: (id: string) =>
    fetch(`/api/cards/${id}`, { method: 'DELETE', headers: authHeaders() }).then((r) => {
      if (!r.ok) throw new ApiError(r.status, `delete failed: ${r.status}`)
    }),

  listCardHistory: (id: string) =>
    fetch(`/api/cards/${id}/history`).then(json<{ version: number; savedAt: string }[]>),

  getCardHistoryVersion: (id: string, version: number) =>
    fetch(`/api/cards/${id}/history/${version}`).then(json<{ version: number; savedAt: string; body: import('./types').Card }>),

  listUsers: () =>
    fetch('/api/users', { headers: authHeaders() }).then(
      json<Array<{ username: string; role: string; mustChangePassword: boolean; createdAt: string }>>
    ),

  createUser: (username: string, password: string, role: 'admin' | 'editor' = 'editor', mustChangePassword = false) =>
    fetch('/api/users', {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ username, password, role, mustChangePassword }),
    }).then(json<{ username: string; role: string }>),

  changePassword: (currentPassword: string, newPassword: string) =>
    fetch('/api/me/password', {
      method: 'PUT',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
    }).then(json<{ ok: boolean }>),

  deleteImages: (paths: string[]) =>
    fetch('/api/images', {
      method: 'DELETE',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ paths }),
    }).then((r) => { if (!r.ok) throw new ApiError(r.status, `deleteImages failed: ${r.status}`) }),

  adminStats: () =>
    fetch('/api/admin/stats', { headers: authHeaders() }).then(
      json<{ cardCount: number; imageCount: number; fileCount: number; uploadsDirBytes: number; dbBytes: number }>
    ),

  adminScanOrphans: () =>
    fetch('/api/admin/orphans', { headers: authHeaders() }).then(
      json<{ count: number; paths: string[] }>
    ),

  adminDeleteOrphans: () =>
    fetch('/api/admin/orphans', { method: 'DELETE', headers: authHeaders() }).then(
      json<{ moved: number }>
    ),

  adminListTrash: () =>
    fetch('/api/admin/trash', { headers: authHeaders() }).then(
      json<{
        entries: Array<{
          id: number | null
          trashFilename: string
          originalPath: string | null
          originalThumbPath?: string | null
          originalStagePath?: string | null
          cardId?: string | null
          reason: 'orphan' | 'user_delete' | 'unknown'
          trashedAt: string | null
          onDisk: boolean
          bytes: number
        }>
        totalBytes: number
      }>
    ),

  adminDeleteTrash: (opts: { ids?: number[]; all?: boolean; unknown?: boolean }) =>
    fetch('/api/admin/trash', {
      method: 'DELETE',
      headers: { ...authHeaders(), 'content-type': 'application/json' },
      body: JSON.stringify(opts),
    }).then(json<{ deleted: number }>),

  adminListAudit: (opts?: { limit?: number; beforeId?: number }) => {
    const params = new URLSearchParams()
    if (opts?.limit) params.set('limit', String(opts.limit))
    if (opts?.beforeId) params.set('before_id', String(opts.beforeId))
    const qs = params.toString()
    return fetch(`/api/admin/audit${qs ? `?${qs}` : ''}`, { headers: authHeaders() }).then(
      json<Array<{
        id: number
        username: string
        action: string
        entity: string
        entityId: string | null
        detail: unknown | null
        createdAt: string
      }>>
    )
  },

  adminRestoreTrash: (ids: number[]) =>
    fetch('/api/admin/trash/restore', {
      method: 'POST',
      headers: { ...authHeaders(), 'content-type': 'application/json' },
      body: JSON.stringify({ ids }),
    }).then(json<{ restored: number; imageIds: number[] }>),

  adminRepairFigurePaths: () =>
    fetch('/api/admin/repair-figure-paths', { method: 'POST', headers: authHeaders() }).then(
      json<{ repaired: number; cleared: number }>
    ),

  adminListDeletedCards: () =>
    fetch('/api/admin/deleted-cards', { headers: authHeaders() }).then(
      json<{ cards: Array<{ id: string; name: string; deletedAt: string }> }>
    ),

  adminRestoreCard: (id: string) =>
    fetch(`/api/admin/deleted-cards/${id}/restore`, { method: 'POST', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  adminPurgeCard: (id: string) =>
    fetch(`/api/admin/deleted-cards/${id}`, { method: 'DELETE', headers: authHeaders() }).then(
      r => { if (!r.ok) throw new ApiError(r.status, `purge failed: ${r.status}`) }
    ),

  adminExportSeed: () =>
    fetch('/api/admin/export-seed', { method: 'POST', headers: authHeaders() }).then(
      json<{ exported: number }>
    ),

  adminReloadSeed: () =>
    fetch('/api/admin/reload-seed', { method: 'POST', headers: authHeaders() }).then(
      json<{ upserted: number; removed: number }>
    ),

  submitSuggestion: (payload: { cardId: string; title: string; credit?: string; adjustments: object[] }) =>
    fetch('/api/suggestions', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ card_id: payload.cardId, title: payload.title, credit: payload.credit, adjustments: payload.adjustments }),
    }).then(json<{ ok: boolean }>),

  adminListSuggestions: () =>
    fetch('/api/admin/suggestions', { headers: authHeaders() }).then(
      json<{ id: number; cardId: string; title: string; credit: string | null; adjustments: object[]; submittedAt: string; ip: string; status: 'pending' | 'liked' }[]>
    ),

  adminDismissSuggestion: (id: number) =>
    fetch(`/api/admin/suggestions/${id}`, { method: 'DELETE', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  adminLikeSuggestion: (id: number) =>
    fetch(`/api/admin/suggestions/${id}`, { method: 'PATCH', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  listTuningPresets: () =>
    fetch('/api/tuning-presets').then(
      json<{ id: number; name: string; values: Record<string, number>; kind: 'build' | 'baseline'; upgrades: UpgradeCategory[]; baselineId: number | null; createdAt: string }[]>
    ),

  createTuningPreset: (payload: { name: string; values: Record<string, number>; kind?: string; upgrades?: UpgradeCategory[]; baselineId?: number | null }) =>
    fetch('/api/tuning-presets', {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify(payload),
    }).then(json<{ id: number; name: string; values: Record<string, number>; kind: 'build' | 'baseline'; upgrades: UpgradeCategory[]; baselineId: number | null; createdAt: string }>),

  deleteTuningPreset: (id: number) =>
    fetch(`/api/tuning-presets/${id}`, { method: 'DELETE', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  getTheme: () =>
    fetch('/api/theme').then(json<Record<string, unknown>>),

  putTheme: (body: Record<string, unknown>) =>
    fetch('/api/theme', {
      method: 'PUT',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify(body),
    }).then(json<Record<string, unknown>>),

  // Upload a file with card context for folder naming; returns original + variant paths.
  // fileIndex: when set, the backend uses it for sequential filename (001.jpg, 002.jpg…)
  uploadImage: (
    file: File,
    card: { name: string; subtitle: string; collections: string[]; id?: string },
    fileIndex?: number,
    carId?: string,
    imageRole?: string,
  ) => {
    const fd = new FormData()
    fd.append('cardName', card.name)
    fd.append('cardSubtitle', card.subtitle)
    fd.append('cardCollections', card.collections.join(','))
    if (card.id) fd.append('cardId', card.id)
    if (carId) fd.append('carId', carId)
    if (fileIndex !== undefined) fd.append('fileIndex', String(fileIndex))
    if (imageRole) fd.append('imageRole', imageRole)
    fd.append('file', file)
    return fetch('/api/images', { method: 'POST', headers: authHeaders(), body: fd }).then(
      json<{ id?: number; path: string; thumbPath?: string; stagePath?: string; carId?: string | null; imageRole?: string }>,
    )
  },

  uploadImageWithProgress: (
    file: File,
    card: { name: string; subtitle: string; collections: string[]; id?: string },
    options: { fileIndex?: number; carId?: string; liveryId?: number; imageRole?: string },
    onProgress: (pct: number) => void,
  ): Promise<{ id?: number; path: string; thumbPath?: string; stagePath?: string; carId?: string | null; imageRole?: string }> => {
    return new Promise((resolve, reject) => {
      const fd = new FormData()
      fd.append('cardName', card.name)
      fd.append('cardSubtitle', card.subtitle)
      fd.append('cardCollections', card.collections.join(','))
      if (card.id) fd.append('cardId', card.id)
      if (options.carId) fd.append('carId', options.carId)
      if (options.liveryId !== undefined) fd.append('liveryId', String(options.liveryId))
      if (options.fileIndex !== undefined) fd.append('fileIndex', String(options.fileIndex))
      if (options.imageRole) fd.append('imageRole', options.imageRole)
      fd.append('file', file)
      const xhr = new XMLHttpRequest()
      xhr.open('POST', '/api/images')
      const t = localStorage.getItem('auth_token')
      if (t) xhr.setRequestHeader('Authorization', `Bearer ${t}`)
      xhr.upload.onprogress = (e) => { if (e.lengthComputable) onProgress(Math.round(e.loaded / e.total * 100)) }
      xhr.onload = () => xhr.status >= 200 && xhr.status < 300
        ? resolve(JSON.parse(xhr.responseText))
        : reject(new Error(`Upload failed: ${xhr.status}`))
      xhr.onerror = () => reject(new Error('Upload failed'))
      xhr.send(fd)
    })
  },

  assessLiveryColor: (id: number) =>
    fetch(`/api/admin/liveries/${id}/assess-color`, { method: 'POST', headers: authHeaders() }).then(
      json<{ id: number; serial: string; primary: string; secondary?: string }>
    ),

  migrateImages: (imageIds: number[], carId: string, liveryId: number) =>
    fetch('/api/admin/images/migrate', {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ imageIds, carId, liveryId }),
    }).then(json<{ migrated: Array<{ id: number; path: string; thumbPath: string; stagePath: string; carId: string; liveryId: number }> }>),
}
