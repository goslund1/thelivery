<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useModalStore } from '../stores/modal'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useAuthStore } from '../stores/auth'
import type { OgConfig } from '../types'

const modal = useModalStore()
const store = useCardsStore()
const ui    = useUiStore()
const auth  = useAuthStore()

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
  if (!id) { presets.value = []; selectedPresetId.value = null; previewSrc.value = null; return }
  nextTick(() => {
    const nameParts = [card.value?.name, firstCarName.value].filter(Boolean).join(' — ')
    const code = shareCode.value
    redditTitle.value = code ? `${nameParts} | Share code: ${code}` : nameParts
  })
  fetchPresets()
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

// ── OG design section ────────────────────────────────────────────────────────

interface OgPreset { id: number; name: string; config: OgConfig }

const presets          = ref<OgPreset[]>([])
const selectedPresetId = ref<number | null>(null)
const previewSrc       = ref<string | null>(null)
const previewLoading   = ref(false)

const selectedPreset = computed(() =>
  presets.value.find(p => p.id === selectedPresetId.value) ?? null
)

const cardOverlayConfig = computed(() => card.value?.shareOverlayConfig ?? null)

async function fetchPresets() {
  const res = await fetch('/api/og-presets')
  if (res.ok) presets.value = await res.json()
}

async function selectPreset(id: number) {
  selectedPresetId.value = id
  const preset = presets.value.find(p => p.id === id)
  if (!preset || !card.value?.images[0]) return
  await fetchPreview({ ...preset.config, photoId: card.value.images[0].id })
}

async function fetchPreview(config: OgConfig) {
  previewLoading.value = true
  previewSrc.value = null
  try {
    const token = localStorage.getItem('auth_token') ?? ''
    const res = await fetch('/share/preview', {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
      body: JSON.stringify(config),
    })
    if (res.ok) {
      const blob = await res.blob()
      previewSrc.value = URL.createObjectURL(blob)
    }
  } finally {
    previewLoading.value = false
  }
}

async function usePreset() {
  const c = card.value
  const preset = selectedPreset.value
  if (!c || !preset) return
  const cfg: OgConfig = { ...preset.config, photoId: c.images[0]?.id ?? preset.config.photoId }
  c.shareOverlayConfig = cfg
  await store.save(c.id)
  selectedPresetId.value = null
  previewSrc.value = null
}

async function resetOverlay() {
  const c = card.value
  if (!c) return
  delete c.shareOverlayConfig
  await store.save(c.id)
  previewSrc.value = null
}

function openOgMakerBlank() {
  const c = card.value
  if (!c) return
  modal.openOgMaker({
    photoId: c.images[0]?.id ?? null,
    photos: c.images.filter(img => img.included !== false),
    boxes: [],
    cardId: c.id,
  })
}

function openOgMakerWithPreset() {
  const c = card.value
  const preset = selectedPreset.value
  if (!c || !preset) return
  modal.openOgMaker({
    photoId: c.images[0]?.id ?? null,
    photos: c.images.filter(img => img.included !== false),
    boxes: preset.config.textBoxes.map(b => ({ ...b, id: crypto.randomUUID() })),
    cardId: c.id,
    presetName: preset.name,
  })
}

function openOgMakerWithCurrent() {
  const c = card.value
  if (!c || !c.shareOverlayConfig) return
  modal.openOgMaker({
    photoId: c.shareOverlayConfig.photoId,
    photos: c.images.filter(img => img.included !== false),
    boxes: c.shareOverlayConfig.textBoxes.map(b => ({ ...b, id: crypto.randomUUID() })),
    cardId: c.id,
  })
}

// Show live preview of current card overlay when ShareModal opens
watch(cardOverlayConfig, (cfg) => {
  if (cfg) fetchPreview(cfg)
}, { immediate: true })
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

        <!-- OG design section (admin only) -->
        <div v-if="auth.isAuthenticated" class="share-og-section">
          <div class="share-og-label">Share card design</div>

          <!-- Card already has a saved overlay -->
          <template v-if="cardOverlayConfig">
            <div class="share-og-current">
              <img v-if="previewSrc" :src="previewSrc" class="share-og-preview" />
              <div v-else-if="previewLoading" class="share-og-preview share-og-preview--loading" />
            </div>
            <div class="share-og-actions">
              <button class="share-og-btn share-og-btn--adjust" @click="openOgMakerWithCurrent">Adjust</button>
              <button class="share-og-btn share-og-btn--reset" @click="resetOverlay">Reset</button>
            </div>
          </template>

          <!-- No overlay yet — show preset picker -->
          <template v-else>
            <div v-if="presets.length" class="share-og-presets">
              <button
                v-for="p in presets"
                :key="p.id"
                class="share-og-preset-chip"
                :class="{ 'share-og-preset-chip--active': selectedPresetId === p.id }"
                @click="selectPreset(p.id)"
              >{{ p.name }}</button>
            </div>
            <div v-else class="share-og-empty">No presets saved yet.</div>

            <!-- Preview of selected preset -->
            <div v-if="selectedPresetId" class="share-og-current">
              <img v-if="previewSrc" :src="previewSrc" class="share-og-preview" />
              <div v-else-if="previewLoading" class="share-og-preview share-og-preview--loading" />
            </div>

            <div class="share-og-actions">
              <template v-if="selectedPresetId">
                <button class="share-og-btn share-og-btn--use" @click="usePreset">Use This</button>
                <button class="share-og-btn share-og-btn--adjust" @click="openOgMakerWithPreset">Adjust</button>
              </template>
              <button class="share-og-btn share-og-btn--blank" @click="openOgMakerBlank">Start Blank</button>
            </div>
          </template>
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

.share-og-section {
  padding: 10px 16px;
  border-top: 1px solid var(--panel-edge);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.share-og-label {
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.share-og-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.share-og-preset-chip {
  padding: 3px 10px;
  border-radius: 12px;
  border: 1px solid var(--panel-edge);
  background: transparent;
  color: var(--text);
  font-size: 0.78rem;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.share-og-preset-chip:hover { border-color: var(--accent); }
.share-og-preset-chip--active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  color: var(--accent);
}
.share-og-empty { font-size: 0.8rem; color: var(--muted); }
.share-og-current { width: 100%; }
.share-og-preview {
  width: 100%;
  aspect-ratio: 1200 / 630;
  object-fit: cover;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  display: block;
}
.share-og-preview--loading {
  background: var(--panel-edge);
  animation: og-pulse 0.8s ease-in-out infinite alternate;
}
@keyframes og-pulse { from { opacity: 0.4 } to { opacity: 0.8 } }
.share-og-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.share-og-btn {
  padding: 4px 12px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: transparent;
  color: var(--text);
  font-size: 0.78rem;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.share-og-btn:hover { border-color: var(--accent); color: var(--accent); }
.share-og-btn--use    { border-color: var(--accent); color: var(--accent); }
.share-og-btn--adjust { border-color: var(--highlight); color: var(--highlight); }
.share-og-btn--reset  { border-color: var(--muted); color: var(--muted); }
.share-og-btn--blank  { border-color: var(--panel-edge); }
</style>
