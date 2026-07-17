<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useModalStore } from '../stores/modal'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'

const modal = useModalStore()
const store = useCardsStore()
const ui    = useUiStore()

const card = computed(() => {
  if (!modal.shareCardId) return null
  return store.cards.find(c => c.id === modal.shareCardId) ?? null
})

function slugify(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')
}

const recipe = computed(() => {
  const c = card.value
  if (!c) return null
  return c.sections.find(s => s.type === 'forza_recipe') ?? null
})

const firstCarName = computed(() => {
  const r = recipe.value
  return r && 'cars' in r ? ((r as any).cars?.[0]?.carName ?? '') : ''
})

const shareCode = computed(() => {
  const r = recipe.value
  return r && 'shareCode' in r ? ((r as any).shareCode ?? '') : ''
})

const shareUrl = computed(() => {
  const c = card.value
  if (!c) return ''
  const parts = [slugify(c.name), slugify(firstCarName.value)].filter(Boolean)
  const slug = parts.join('-')
  return `${window.location.origin}/share/${c.id}${slug ? '/' + slug : ''}`
})

const copied = ref(false)
const redditTitle = ref('')

watch(() => modal.shareCardId, (id) => {
  if (!id) return
  nextTick(() => {
    const nameParts = [card.value?.name, firstCarName.value].filter(Boolean).join(' — ')
    const code = shareCode.value
    redditTitle.value = code ? `${nameParts} | Share code: ${code}` : nameParts
  })
})

async function copyLink() {
  if (!shareUrl.value) return
  await navigator.clipboard.writeText(shareUrl.value)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}

function openReddit() {
  const url = encodeURIComponent(shareUrl.value)
  const title = encodeURIComponent(redditTitle.value)
  window.open(`https://www.reddit.com/submit?url=${url}&title=${title}`, '_blank', 'noopener')
}

function openOgMaker() {
  const c = card.value
  if (!c) return
  const leadId = c.images[0]?.id ?? null
  modal.openOgMaker({
    photoId: leadId,
    photos: c.images.filter(img => img.included !== false),
    boxes: [],
    presetName: '',
  })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modal.shareCardId" class="share-backdrop" @click.self="modal.closeShare()">
      <div class="share-panel">
        <div class="share-header">
          <span class="share-title">Share</span>
          <button class="share-close" aria-label="Close" @click="modal.closeShare()">×</button>
        </div>

        <div class="share-url-row">
          <span class="share-url-text">{{ shareUrl }}</span>
          <button class="share-copy-btn" :class="{ copied }" @click="copyLink">
            {{ copied ? 'Copied!' : 'Copy Link' }}
          </button>
        </div>

        <div class="share-destinations">
          <div class="share-reddit">
            <div class="share-reddit-title-row">
              <label class="share-reddit-label">Post title</label>
            </div>
            <input
              v-model="redditTitle"
              class="share-reddit-input"
              maxlength="300"
              placeholder="Post title…"
            />
            <button class="share-dest-btn share-dest-btn--reddit" @click="openReddit">
              <span class="share-dest-icon">🔺</span>
              <span class="share-dest-label">Post to Reddit</span>
              <span class="share-dest-tag share-dest-tag--arrow">→</span>
            </button>
          </div>
          <button class="share-dest-btn share-dest-btn--soon" disabled>
            <span class="share-dest-icon">💬</span>
            <span class="share-dest-label">Discord</span>
            <span class="share-dest-tag">Coming soon</span>
          </button>
        </div>

        <div v-if="ui.isEditing" class="share-og-row">
          <button class="share-og-btn" @click="openOgMaker">
            <span>🎨</span>
            <span>Design Share Card</span>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.share-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1200;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
}

.share-panel {
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 8px;
  padding: 22px 24px;
  width: 420px;
  max-width: 92vw;
  box-shadow: 0 8px 40px rgba(0,0,0,0.5);
}

.share-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.share-title {
  font-family: 'Archivo Black', sans-serif;
  font-size: 14px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--fg);
}

.share-close {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  padding: 0 2px;
  transition: color 0.12s;
}
.share-close:hover { color: var(--fg); }

.share-url-row {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 5px;
  padding: 8px 10px;
  margin-bottom: 18px;
}

.share-url-text {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--muted-light);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.share-copy-btn {
  flex-shrink: 0;
  padding: 5px 12px;
  border-radius: 4px;
  border: 1px solid var(--accent);
  background: transparent;
  color: var(--accent);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  white-space: nowrap;
}
.share-copy-btn:hover { background: var(--accent); color: var(--panel); }
.share-copy-btn.copied { background: var(--accent); color: var(--panel); }

.share-destinations {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.share-dest-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 14px;
  border-radius: 5px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s;
}
.share-dest-btn--soon {
  cursor: not-allowed;
  opacity: 0.5;
}

.share-dest-icon { font-size: 16px; line-height: 1; }

.share-dest-label {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  color: var(--fg);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.share-dest-tag {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.share-dest-tag--arrow {
  font-size: 14px;
  letter-spacing: 0;
  text-transform: none;
}

.share-reddit {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.share-reddit-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.share-reddit-input {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 10px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  outline: none;
  transition: border-color 0.15s;
}
.share-reddit-input:focus { border-color: var(--accent); }

.share-dest-btn--reddit {
  color: var(--fg);
  transition: border-color 0.15s, background 0.15s;
}
.share-dest-btn--reddit:hover {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}

.share-og-row {
  padding: 10px 16px;
  border-top: 1px solid var(--panel-edge);
}
.share-og-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  background: transparent;
  border: 1px dashed var(--panel-edge);
  border-radius: 4px;
  color: var(--muted);
  font-size: 0.8rem;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.share-og-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
