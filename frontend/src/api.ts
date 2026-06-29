// Thin API client for the Rust backend. Uses relative URLs (dev server proxies
// /api and /uploads to :8787; in prod they are same-origin). Mutating calls send
// the JWT as a Bearer header; reads are public.
import type { Card } from './types'

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
    }).then(json<{ token: string; username: string }>),

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

  createUser: (username: string, password: string) =>
    fetch('/api/users', {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ username, password }),
    }).then(json<{ username: string }>),

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
      json<{ deleted: number }>
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
      json<{ id: number; cardId: string; title: string; credit: string | null; adjustments: object[]; submittedAt: string; ip: string; reviewed: boolean }[]>
    ),

  adminDismissSuggestion: (id: number) =>
    fetch(`/api/admin/suggestions/${id}`, { method: 'DELETE', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  listTuningPresets: () =>
    fetch('/api/tuning-presets').then(
      json<{ id: number; name: string; values: Record<string, number>; createdAt: string }[]>
    ),

  createTuningPreset: (payload: { name: string; values: Record<string, number> }) =>
    fetch('/api/tuning-presets', {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...authHeaders() },
      body: JSON.stringify(payload),
    }).then(json<{ id: number; name: string; values: Record<string, number> }>),

  deleteTuningPreset: (id: number) =>
    fetch(`/api/tuning-presets/${id}`, { method: 'DELETE', headers: authHeaders() }).then(
      json<{ ok: boolean }>
    ),

  // Upload a file with card context for folder naming; returns original + variant paths.
  // fileIndex: when set, the backend uses it for sequential filename (001.jpg, 002.jpg…)
  uploadImage: (
    file: File,
    card: { name: string; subtitle: string; collections: string[] },
    fileIndex?: number,
  ) => {
    const fd = new FormData()
    fd.append('cardName', card.name)
    fd.append('cardSubtitle', card.subtitle)
    fd.append('cardCollections', card.collections.join(','))
    if (fileIndex !== undefined) fd.append('fileIndex', String(fileIndex))
    fd.append('file', file)
    return fetch('/api/images', { method: 'POST', headers: authHeaders(), body: fd }).then(
      json<{ path: string; thumbPath?: string; stagePath?: string }>,
    )
  },
}
