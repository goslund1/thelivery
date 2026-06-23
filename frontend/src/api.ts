// Thin API client for the Rust backend. Uses relative URLs (dev server proxies
// /api and /uploads to :8787; in prod they are same-origin).
import type { Card } from './types'

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${await res.text()}`)
  return res.json() as Promise<T>
}

export const api = {
  listCards: () => fetch('/api/cards').then(json<Card[]>),

  getCard: (id: string) => fetch(`/api/cards/${id}`).then(json<Card>),

  saveCard: (card: Card) =>
    fetch(`/api/cards/${card.id}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(card),
    }).then(json<Card>),

  createCard: (card: Card) =>
    fetch('/api/cards', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(card),
    }).then(json<Card>),

  deleteCard: (id: string) =>
    fetch(`/api/cards/${id}`, { method: 'DELETE' }).then((r) => {
      if (!r.ok) throw new Error(`delete failed: ${r.status}`)
    }),

  deleteImages: (paths: string[]) =>
    fetch('/api/images', {
      method: 'DELETE',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ paths }),
    }).then((r) => { if (!r.ok) throw new Error(`deleteImages failed: ${r.status}`) }),

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
    return fetch('/api/images', { method: 'POST', body: fd }).then(
      json<{ path: string; thumbPath?: string; stagePath?: string }>
    )
  },
}
