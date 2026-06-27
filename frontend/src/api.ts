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

  // Upload a file, returns its served URL path.
  uploadImage: (file: File) => {
    const fd = new FormData()
    fd.append('file', file)
    return fetch('/api/images', { method: 'POST', headers: authHeaders(), body: fd }).then(
      json<{ path: string }>,
    )
  },
}
