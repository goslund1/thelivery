// Thin API client for the Rust backend. Uses relative URLs (dev server proxies
// /api and /uploads to :8787; in prod they are same-origin).
import type { Livery } from './types'

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${await res.text()}`)
  return res.json() as Promise<T>
}

export const api = {
  listLiveries: () => fetch('/api/liveries').then(json<Livery[]>),

  getLivery: (id: string) => fetch(`/api/liveries/${id}`).then(json<Livery>),

  saveLivery: (livery: Livery) =>
    fetch(`/api/liveries/${livery.id}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(livery),
    }).then(json<Livery>),

  createLivery: (livery: Livery) =>
    fetch('/api/liveries', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(livery),
    }).then(json<Livery>),

  deleteLivery: (id: string) =>
    fetch(`/api/liveries/${id}`, { method: 'DELETE' }).then((r) => {
      if (!r.ok) throw new Error(`delete failed: ${r.status}`)
    }),

  // Upload a file, returns its served URL path.
  uploadImage: (file: File) => {
    const fd = new FormData()
    fd.append('file', file)
    return fetch('/api/images', { method: 'POST', body: fd }).then(json<{ path: string }>)
  },
}
