<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Card, AdjustmentRow, UpgradeCategory } from '../types'
import { api } from '../api'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useScrollLock } from '../composables/useScrollLock'

const props = defineProps<{ cardId: string }>()
const { lockScroll, unlockScroll } = useScrollLock()
const cards = useCardsStore()
const ui = useUiStore()
const modal = useModalStore()

// ── Version list ──────────────────────────────────────────────────────────────

interface VersionEntry { version: number; savedAt: string }
const versions = ref<VersionEntry[]>([])
const loading = ref(false)
const loadError = ref<string | null>(null)

async function loadVersions() {
  loading.value = true
  loadError.value = null
  try {
    versions.value = await api.listCardHistory(props.cardId)
  } catch (e) {
    loadError.value = (e as Error).message
  } finally {
    loading.value = false
  }
}

watch(() => props.cardId, loadVersions, { immediate: true })

// ── Selected version ──────────────────────────────────────────────────────────

const selected = ref<{ version: number; savedAt: string; body: Card } | null>(null)
const loadingVersion = ref(false)

async function selectVersion(v: VersionEntry) {
  if (selected.value?.version === v.version) { selected.value = null; return }
  loadingVersion.value = true
  try {
    selected.value = await api.getCardHistoryVersion(props.cardId, v.version)
  } finally {
    loadingVersion.value = false
  }
}

// ── Diff ──────────────────────────────────────────────────────────────────────

const current = computed(() => cards.byId(props.cardId))

interface SliderDiff { key: string; label: string; tab: string; old: number; cur: number }
interface UpgradeDiff { part: string; change: 'added' | 'removed' }
interface SpecDiff    { key: string; old: string; cur: string }
interface TextDiff    { label: string }

const diff = computed(() => {
  const hist = selected.value?.body
  const cur  = current.value
  if (!hist || !cur) return null

  // ── Adjustments (sliders) ──────────────────────────────────────────────────
  const sliders: SliderDiff[] = []
  const histRecipe = hist.sections.find(s => s.type === 'forza_recipe')
  const curRecipe  = cur.sections.find(s => s.type === 'forza_recipe')
  const histAdj: AdjustmentRow[] = histRecipe?.type === 'forza_recipe' ? histRecipe.adjustments : []
  const curAdj:  AdjustmentRow[] = curRecipe?.type  === 'forza_recipe' ? curRecipe.adjustments  : []

  const histAdjMap = new Map(histAdj.map(r => [r.key, r]))
  const curAdjMap  = new Map(curAdj.map(r => [r.key, r]))
  const allKeys    = new Set([...histAdjMap.keys(), ...curAdjMap.keys()])

  for (const key of allKeys) {
    const h = histAdjMap.get(key)
    const c = curAdjMap.get(key)
    const oldVal = h?.value ?? null
    const curVal = c?.value ?? null
    if (oldVal !== curVal) {
      sliders.push({
        key,
        label: h?.label ?? c?.label ?? key,
        tab:   h?.tab   ?? c?.tab   ?? '',
        old:   oldVal as number,
        cur:   curVal as number,
      })
    }
  }

  // ── Upgrades ───────────────────────────────────────────────────────────────
  const upgrades: UpgradeDiff[] = []
  const histUpgrades: UpgradeCategory[] = histRecipe?.type === 'forza_recipe' ? histRecipe.upgrades : []
  const curUpgrades:  UpgradeCategory[] = curRecipe?.type  === 'forza_recipe' ? curRecipe.upgrades  : []
  const histParts = new Set(histUpgrades.flatMap(c => c.parts))
  const curParts  = new Set(curUpgrades.flatMap(c => c.parts))
  for (const p of histParts) if (!curParts.has(p)) upgrades.push({ part: p, change: 'removed' })
  for (const p of curParts)  if (!histParts.has(p)) upgrades.push({ part: p, change: 'added' })

  // ── Core specs ─────────────────────────────────────────────────────────────
  const specs: SpecDiff[] = []
  const histSpecs = histRecipe?.type === 'forza_recipe' ? (histRecipe.coreSpecs ?? {}) : {}
  const curSpecs  = curRecipe?.type  === 'forza_recipe' ? (curRecipe.coreSpecs  ?? {}) : {}
  const allSpecKeys = new Set([...Object.keys(histSpecs), ...Object.keys(curSpecs)])
  for (const k of allSpecKeys) {
    if ((histSpecs[k] ?? '') !== (curSpecs[k] ?? ''))
      specs.push({ key: k, old: histSpecs[k] ?? '', cur: curSpecs[k] ?? '' })
  }

  // ── Text sections ──────────────────────────────────────────────────────────
  const textChanged: TextDiff[] = []
  for (const hs of hist.sections) {
    if (hs.type !== 'text') continue
    const cs = cur.sections.find(s => s.key === hs.key && s.type === 'text')
    if (!cs || cs.type !== 'text') continue
    if (hs.body !== cs.body) textChanged.push({ label: hs.label })
  }

  // ── Top-level fields ───────────────────────────────────────────────────────
  const meta: string[] = []
  if (hist.name !== cur.name) meta.push(`Name: "${hist.name}" → "${cur.name}"`)
  if ((hist.subtitle ?? '') !== (cur.subtitle ?? '')) meta.push('Subtitle changed')
  if ((hist.liveryShareCode ?? '') !== (cur.liveryShareCode ?? '')) meta.push('Livery share code changed')

  const tuneDiffs: string[] = []
  const hTuneName = histRecipe?.type === 'forza_recipe' ? histRecipe.tuneName : ''
  const cTuneName = curRecipe?.type  === 'forza_recipe' ? curRecipe.tuneName  : ''
  if (hTuneName !== cTuneName) tuneDiffs.push('Tune name changed')
  const hCode = histRecipe?.type === 'forza_recipe' ? histRecipe.shareCode : ''
  const cCode = curRecipe?.type  === 'forza_recipe' ? curRecipe.shareCode  : ''
  if (hCode !== cCode) tuneDiffs.push('Tune share code changed')

  return { sliders, upgrades, specs, textChanged, meta, tuneDiffs }
})

const hasChanges = computed(() =>
  diff.value && (
    diff.value.sliders.length > 0 ||
    diff.value.upgrades.length > 0 ||
    diff.value.specs.length > 0 ||
    diff.value.textChanged.length > 0 ||
    diff.value.meta.length > 0 ||
    diff.value.tuneDiffs.length > 0
  )
)

// ── Restore ───────────────────────────────────────────────────────────────────

const restoring = ref(false)
const restoreConfirm = ref(false)

async function doRestore() {
  if (!selected.value) return
  restoring.value = true
  try {
    cards.restoreCardVersion(props.cardId, selected.value.body)
    ui.markCardDirty(props.cardId)
    restoreConfirm.value = true
    setTimeout(() => { restoreConfirm.value = false }, 2500)
    selected.value = null
    await loadVersions()
  } finally {
    restoring.value = false
  }
}

// ── Formatting ────────────────────────────────────────────────────────────────

function formatDate(iso: string) {
  const d = new Date(iso)
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) +
    ' · ' + d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })
}
function fmtVal(v: number | null) {
  if (v === null) return '—'
  return Number.isInteger(v) ? String(v) : v.toFixed(2)
}
</script>

<template>
  <div class="ch-backdrop" @click.self="modal.closeHistory()">
    <div class="ch-modal" @mouseenter="lockScroll" @mouseleave="unlockScroll">

      <div class="ch-header">
        <span class="ch-title">Version History</span>
        <span class="ch-card-name">{{ current?.name }}</span>
        <button class="ch-close" @click="modal.closeHistory()">×</button>
      </div>

      <div class="ch-body">

        <!-- Version list -->
        <div class="ch-sidebar">
          <p v-if="loading" class="ch-muted">Loading…</p>
          <p v-else-if="loadError" class="ch-muted ch-error">{{ loadError }}</p>
          <p v-else-if="!versions.length" class="ch-muted">No saved versions yet.</p>
          <ul v-else class="ch-version-list">
            <li
              v-for="v in versions"
              :key="v.version"
              class="ch-version-item"
              :class="{ active: selected?.version === v.version }"
              @click="selectVersion(v)"
            >
              <span class="ch-version-num">v{{ v.version }}</span>
              <span class="ch-version-date">{{ formatDate(v.savedAt) }}</span>
            </li>
          </ul>
        </div>

        <!-- Diff panel -->
        <div class="ch-diff">
          <div v-if="!selected && !loadingVersion" class="ch-diff-empty">
            <p class="ch-muted">Select a version to compare</p>
          </div>
          <div v-else-if="loadingVersion" class="ch-diff-empty">
            <p class="ch-muted">Loading…</p>
          </div>
          <template v-else-if="selected && diff">

            <div class="ch-diff-header">
              <span>v{{ selected.version }} &nbsp;·&nbsp; {{ formatDate(selected.savedAt) }}</span>
              <button
                v-if="hasChanges"
                class="ch-restore-btn"
                :disabled="restoring"
                @click="doRestore"
              >{{ restoring ? 'Restoring…' : 'Restore to this version' }}</button>
              <span v-if="restoreConfirm" class="ch-restore-ok">Restored — save to persist</span>
            </div>

            <p v-if="!hasChanges" class="ch-muted ch-no-diff">No differences from current state.</p>

            <!-- Sliders -->
            <section v-if="diff.sliders.length" class="ch-section">
              <h4 class="ch-section-title">Slider values ({{ diff.sliders.length }})</h4>
              <table class="ch-table">
                <thead><tr><th>Tab</th><th>Slider</th><th>This version</th><th>Current</th></tr></thead>
                <tbody>
                  <tr v-for="s in diff.sliders" :key="s.key">
                    <td class="ch-tab">{{ s.tab }}</td>
                    <td>{{ s.label }}</td>
                    <td class="ch-val-hist">{{ fmtVal(s.old) }}</td>
                    <td class="ch-val-cur">{{ fmtVal(s.cur) }}</td>
                  </tr>
                </tbody>
              </table>
            </section>

            <!-- Upgrades -->
            <section v-if="diff.upgrades.length" class="ch-section">
              <h4 class="ch-section-title">Upgrades ({{ diff.upgrades.length }})</h4>
              <ul class="ch-change-list">
                <li v-for="u in diff.upgrades" :key="u.part" :class="u.change">
                  <span class="ch-badge">{{ u.change }}</span> {{ u.part }}
                </li>
              </ul>
            </section>

            <!-- Specs -->
            <section v-if="diff.specs.length" class="ch-section">
              <h4 class="ch-section-title">Core specs</h4>
              <table class="ch-table">
                <thead><tr><th>Spec</th><th>This version</th><th>Current</th></tr></thead>
                <tbody>
                  <tr v-for="s in diff.specs" :key="s.key">
                    <td>{{ s.key }}</td>
                    <td class="ch-val-hist">{{ s.old || '—' }}</td>
                    <td class="ch-val-cur">{{ s.cur || '—' }}</td>
                  </tr>
                </tbody>
              </table>
            </section>

            <!-- Tune / meta -->
            <section v-if="diff.tuneDiffs.length || diff.meta.length || diff.textChanged.length" class="ch-section">
              <h4 class="ch-section-title">Other changes</h4>
              <ul class="ch-change-list">
                <li v-for="m in [...diff.meta, ...diff.tuneDiffs]" :key="m">{{ m }}</li>
                <li v-for="t in diff.textChanged" :key="t.label">{{ t.label }} section text changed</li>
              </ul>
            </section>

          </template>
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.ch-backdrop {
  position: fixed; inset: 0; z-index: 1150;
  background: rgba(0,0,0,.6);
  display: flex; align-items: center; justify-content: center;
}
.ch-modal {
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  width: min(900px, 95vw);
  max-height: 80vh;
  display: flex; flex-direction: column;
  overflow: hidden;
}
.ch-header {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--panel-edge);
  flex-shrink: 0;
}
.ch-title {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px; text-transform: uppercase; letter-spacing: .1em;
  color: var(--muted);
}
.ch-card-name {
  font-size: 13px; font-weight: 600; color: var(--accent);
  flex: 1;
}
.ch-close {
  background: none; border: none; color: var(--muted);
  font-size: 20px; cursor: pointer; padding: 0 4px; line-height: 1;
}
.ch-close:hover { color: var(--text); }

.ch-body {
  display: flex; flex: 1; overflow: hidden;
}

/* Sidebar */
.ch-sidebar {
  width: 210px; flex-shrink: 0;
  border-right: 1px solid var(--panel-edge);
  overflow-y: auto; overscroll-behavior: contain; padding: 10px 0;
}
.ch-version-list { list-style: none; margin: 0; padding: 0; }
.ch-version-item {
  display: flex; flex-direction: column; gap: 2px;
  padding: 10px 14px; cursor: pointer;
  border-left: 3px solid transparent;
  transition: background .12s;
}
.ch-version-item:hover { background: var(--panel-hover, rgba(255,255,255,.04)); }
.ch-version-item.active {
  border-left-color: var(--accent);
  background: var(--panel-hover, rgba(255,255,255,.06));
}
.ch-version-num {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px; color: var(--accent); font-weight: 600;
}
.ch-version-date { font-size: 10px; color: var(--muted); }

/* Diff panel */
.ch-diff {
  flex: 1; overflow-y: auto; overscroll-behavior: contain; padding: 16px 20px;
  display: flex; flex-direction: column; gap: 16px;
}
.ch-diff-empty {
  flex: 1; display: flex; align-items: center; justify-content: center;
}
.ch-muted { color: var(--muted); font-size: 12px; }
.ch-error { color: var(--highlight); }
.ch-no-diff { margin: 0; }

.ch-diff-header {
  display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
  font-size: 11px; color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--panel-edge);
}
.ch-restore-btn {
  margin-left: auto;
  background: transparent;
  border: 1px solid var(--accent);
  border-radius: 4px;
  color: var(--accent);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px; text-transform: uppercase; letter-spacing: .06em;
  padding: 4px 10px; cursor: pointer;
  transition: background .15s, color .15s;
}
.ch-restore-btn:hover:not(:disabled) { background: var(--accent); color: var(--panel); }
.ch-restore-btn:disabled { opacity: .5; cursor: default; }
.ch-restore-ok { font-size: 11px; color: var(--teal, #4ecdc4); }

.ch-section { display: flex; flex-direction: column; gap: 8px; }
.ch-section-title {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px; text-transform: uppercase; letter-spacing: .08em;
  color: var(--muted); margin: 0;
}

.ch-table {
  width: 100%; border-collapse: collapse; font-size: 12px;
}
.ch-table th {
  text-align: left; padding: 4px 8px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px; text-transform: uppercase; letter-spacing: .07em;
  color: var(--muted);
  border-bottom: 1px solid var(--panel-edge);
}
.ch-table td { padding: 5px 8px; border-bottom: 1px solid var(--panel-edge); }
.ch-tab { color: var(--muted); font-size: 11px; }
.ch-val-hist { color: var(--accent); font-family: 'JetBrains Mono', monospace; }
.ch-val-cur  { color: var(--muted); font-family: 'JetBrains Mono', monospace; }

.ch-change-list {
  list-style: none; margin: 0; padding: 0;
  display: flex; flex-direction: column; gap: 5px;
  font-size: 12px;
}
.ch-badge {
  display: inline-block;
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px; text-transform: uppercase; letter-spacing: .06em;
  padding: 1px 5px; border-radius: 3px; margin-right: 6px;
}
.ch-change-list li.added   .ch-badge { background: rgba(78,205,196,.15); color: var(--teal, #4ecdc4); }
.ch-change-list li.removed .ch-badge { background: rgba(255,80,80,.12);  color: var(--highlight); }
</style>
