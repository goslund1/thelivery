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
