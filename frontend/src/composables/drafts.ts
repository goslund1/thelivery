import { ref, watch } from 'vue'
import type { Card } from '../types'
import { useUiStore } from '../stores/ui'
import { useCardsStore } from '../stores/cards'

// Draft safety (decision D5). Unsaved edits live only in memory, so a phone that
// reloads the tab would lose a half-entered tune. Every dirty card is written to
// localStorage as it changes; on the next load the app offers it back. A draft is
// removed once its card is saved or discarded. Storage can be unavailable
// (private mode, full) — every access is guarded and failure just means no draft.

const PREFIX = 'livery:draft:'

export interface Draft {
  id: string
  savedAt: number
  base: string      // fingerprint of the server version the edit started from
  card: Card
}

// Small string hash (djb2) — enough to tell "server copy changed since".
export function fingerprint(json: string | undefined): string {
  if (!json) return ''
  let h = 5381
  for (let i = 0; i < json.length; i++) h = ((h << 5) + h + json.charCodeAt(i)) | 0
  return (h >>> 0).toString(36)
}

export function readDrafts(): Draft[] {
  const out: Draft[] = []
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (!key?.startsWith(PREFIX)) continue
      try { out.push(JSON.parse(localStorage.getItem(key) ?? '') as Draft) } catch { /* corrupt — ignore */ }
    }
  } catch { /* storage unavailable */ }
  return out.sort((a, b) => a.savedAt - b.savedAt)
}

export function clearDraft(id: string) {
  try { localStorage.removeItem(PREFIX + id) } catch { /* ignore */ }
}

function writeDraft(card: Card, base: string) {
  // Images still mid-upload have negative temp ids that won't exist after a reload.
  const clean: Card = { ...card, images: card.images.filter(img => img.id > 0) }
  const draft: Draft = { id: card.id, savedAt: Date.now(), base, card: clean }
  try { localStorage.setItem(PREFIX + card.id, JSON.stringify(draft)) } catch { /* full or blocked */ }
}

// Drafts found at load time, waiting for the restore prompt.
export const pendingDrafts = ref<Draft[]>([])

export function useDraftAutosave() {
  const ui = useUiStore()
  const cards = useCardsStore()
  pendingDrafts.value = readDrafts()

  let timer: ReturnType<typeof setTimeout> | null = null
  watch(
    () => [...ui.dirtyIds].map(id => cards.byId(id)),
    () => {
      if (timer) clearTimeout(timer)
      timer = setTimeout(() => {
        // Only real edits: outside edit mode a card can be marked dirty with no
        // change at all (docs/plan.md, "phantom unsaved card"), and that must
        // not turn into a restore prompt on the next visit.
        if (!ui.isEditing) return
        for (const id of ui.dirtyIds) {
          const c = cards.byId(id)
          if (c) writeDraft(c, fingerprint(cards.baselineOf(id)))
        }
      }, 500)
    },
    { deep: true },
  )
  // A card that stops being dirty was saved or discarded: its draft is done.
  // Only cards dirty in this session are cleared, so drafts from the last visit
  // survive until the restore prompt has been answered.
  watch(() => ui.dirtyIds, (now, before) => {
    for (const id of before ?? []) if (!now.has(id)) clearDraft(id)
  })
}

// Restore: enter edit mode first so the baseline is the server version (Discard
// still means "back to what's saved"), then swap each draft in as the card and
// mark it unsaved.
export function restoreDrafts(drafts: Draft[]) {
  const ui = useUiStore()
  const cards = useCardsStore()
  pendingDrafts.value = []
  if (!ui.isEditing) ui.enterEdit()
  for (const d of drafts) {
    if (!cards.byId(d.id)) { clearDraft(d.id); continue }
    cards.replaceCard(d.id, d.card)
    ui.markCardDirty(d.id)
  }
}

export function discardDrafts(drafts: Draft[]) {
  for (const d of drafts) clearDraft(d.id)
  pendingDrafts.value = []
}
