<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useCardsStore } from '../stores/cards'
import { pendingDrafts, restoreDrafts, discardDrafts, fingerprint } from '../composables/drafts'

// Offers back unsaved edits found in localStorage after a reload (E4). Only when
// signed in (restoring means editing) and once the cards have loaded.
const auth = useAuthStore()
const cards = useCardsStore()

const drafts = computed(() => pendingDrafts.value.filter(d => cards.byId(d.id)))
const open = computed(() => auth.isAuthenticated && !cards.loading && cards.cards.length > 0 && drafts.value.length > 0)

const when = computed(() => {
  const t = Math.max(...drafts.value.map(d => d.savedAt))
  return new Date(t).toLocaleString(undefined, { weekday: 'short', hour: 'numeric', minute: '2-digit' })
})
// The server copy changed since the edit began (saved elsewhere): say so.
function changedOnServer(id: string, base: string) {
  return !!base && fingerprint(JSON.stringify(cards.byId(id))) !== base
}
</script>

<template>
  <div class="conf-overlay" :class="{ open }">
    <div class="conf-panel">
      <div class="conf-head">
        <span>Unsaved Edits Found</span>
      </div>
      <p class="conf-body">
        Edits from {{ when }} weren't saved before the page reloaded:
      </p>
      <ul class="draft-list">
        <li v-for="d in drafts" :key="d.id">
          {{ cards.byId(d.id)?.name || d.card.name }}
          <span v-if="changedOnServer(d.id, d.base)" class="draft-warn">— changed since; restoring replaces that</span>
        </li>
      </ul>
      <div class="conf-actions">
        <button class="conf-btn conf-btn--confirm" @click="restoreDrafts(drafts)">Restore</button>
        <button class="conf-btn conf-btn--discard" @click="discardDrafts(drafts)">Discard</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.draft-list {
  margin: 0 0 14px;
  padding: 0 0 0 18px;
  color: var(--fg);
  font-size: 14px;
  line-height: 1.6;
}
.draft-warn { color: var(--highlight); font-size: 12px; }
</style>
