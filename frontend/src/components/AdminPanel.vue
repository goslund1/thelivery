<script setup lang="ts">
import { ref, computed } from 'vue'
import { useModalStore } from '../stores/modal'
import { useCardsStore } from '../stores/cards'
import { api } from '../api'
import type { Card, ForzaRecipeSection, AdjustmentRow } from '../types'
import { cardToYaml, yamlToCard } from '../utils/cardYaml'
import { useAssessFailures } from '../composables/useAssessFailures'

const { failedAssess } = useAssessFailures()
const modal = useModalStore()
const cards = useCardsStore()

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

type Tab = 'tools' | 'export'
const tab = ref<Tab>('tools')

function close() {
  modal.closeAdminPanel()
  adminError.value = null
  trashResult.value = null
}

// ── Tools: stats / orphans / trash / seed ────────────────────────────────────
type AdminStats = { cardCount: number; imageCount: number; fileCount: number; uploadsDirBytes: number; dbBytes: number }
const adminStats     = ref<AdminStats | null>(null)
const adminStatsBusy = ref(false)
const adminError     = ref<string | null>(null)
const orphanScan     = ref<{ count: number; paths: string[] } | null>(null)
const orphanBusy     = ref(false)
const orphanResult   = ref<string | null>(null)
const repairBusy     = ref(false)
const repairResult   = ref<string | null>(null)
const exportBusy     = ref(false)
const exportResult   = ref<string | null>(null)
const reloadBusy     = ref(false)
const reloadResult   = ref<string | null>(null)

function formatBytes(b: number) {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
  return `${(b / 1024 / 1024).toFixed(1)} MB`
}

async function loadAdminStats() {
  adminStatsBusy.value = true
  adminError.value = null
  try { adminStats.value = await api.adminStats() }
  catch (e) { adminError.value = `Stats failed: ${errMsg(e)}` }
  finally { adminStatsBusy.value = false }
}

async function repairFigurePaths() {
  repairBusy.value = true
  repairResult.value = null
  adminError.value = null
  try {
    const res = await api.adminRepairFigurePaths()
    await useCardsStore().load()
    repairResult.value = res.repaired > 0 || res.cleared > 0
      ? `Repaired ${res.repaired}${res.cleared ? `, cleared ${res.cleared}` : ''}.`
      : 'All figure paths OK.'
  } catch (e) { adminError.value = `Repair failed: ${errMsg(e)}` }
  finally { repairBusy.value = false }
}

async function scanOrphans() {
  orphanBusy.value = true
  orphanResult.value = null
  adminError.value = null
  try { orphanScan.value = await api.adminScanOrphans() }
  catch (e) { adminError.value = `Scan failed: ${errMsg(e)}` }
  finally { orphanBusy.value = false }
}

async function deleteOrphans() {
  orphanBusy.value = true
  adminError.value = null
  try {
    const res = await api.adminDeleteOrphans()
    orphanResult.value = `Moved ${res.moved} file${res.moved !== 1 ? 's' : ''} to trash.`
    orphanScan.value = null
    await loadTrash()
  } catch (e) { adminError.value = `Sweep failed: ${errMsg(e)}` }
  finally { orphanBusy.value = false }
}

async function exportSeed() {
  exportBusy.value = true
  exportResult.value = null
  adminError.value = null
  try {
    const res = await api.adminExportSeed()
    exportResult.value = `Exported ${res.exported} cards to seed file.`
  } catch (e) { adminError.value = `Export failed: ${errMsg(e)}` }
  finally { exportBusy.value = false }
}

async function reloadSeed() {
  reloadBusy.value = true
  reloadResult.value = null
  adminError.value = null
  try {
    const res = await api.adminReloadSeed()
    reloadResult.value = `Reloaded ${res.upserted} cards${res.removed > 0 ? `, removed ${res.removed}` : ''}.`
  } catch (e) { adminError.value = `Reload failed: ${errMsg(e)}` }
  finally { reloadBusy.value = false }
}

// ── Trash ─────────────────────────────────────────────────────────────────────
type TrashEntry = {
  id: number | null
  trashFilename: string
  originalPath: string | null
  cardId?: string | null
  reason: 'orphan' | 'user_delete' | 'unknown'
  trashedAt: string | null
  onDisk: boolean
  bytes: number
}
const trashEntries    = ref<TrashEntry[]>([])
const trashTotalBytes = ref(0)
const trashBusy       = ref(false)
const trashExpanded   = ref(false)
const trashSelected   = ref<Set<number>>(new Set())
const trashResult     = ref<string | null>(null)

async function loadTrash() {
  trashBusy.value = true
  try {
    const res = await api.adminListTrash()
    trashEntries.value = res.entries
    trashTotalBytes.value = res.totalBytes
  } catch (_e) { /* non-fatal */ }
  finally { trashBusy.value = false }
}

function toggleTrashSelect(id: number) {
  const s = new Set(trashSelected.value)
  s.has(id) ? s.delete(id) : s.add(id)
  trashSelected.value = s
}

function toggleSelectAll() {
  const ids = trashEntries.value.filter(e => e.id !== null).map(e => e.id as number)
  trashSelected.value = trashSelected.value.size === ids.length ? new Set() : new Set(ids)
}

async function deleteTrashSelected() {
  const ids = [...trashSelected.value]
  if (!ids.length) return
  trashBusy.value = true; trashResult.value = null; adminError.value = null
  try {
    const res = await api.adminDeleteTrash({ ids })
    trashResult.value = `Permanently deleted ${res.deleted} file${res.deleted !== 1 ? 's' : ''}.`
    trashSelected.value = new Set()
    await loadTrash()
  } catch (e) { adminError.value = `Delete failed: ${errMsg(e)}` }
  finally { trashBusy.value = false }
}

async function deleteAllTrash() {
  trashBusy.value = true; trashResult.value = null; adminError.value = null
  try {
    const res = await api.adminDeleteTrash({ all: true })
    trashResult.value = `Permanently deleted ${res.deleted} file${res.deleted !== 1 ? 's' : ''}.`
    trashSelected.value = new Set()
    trashExpanded.value = false
    await loadTrash()
  } catch (e) { adminError.value = `Delete failed: ${errMsg(e)}` }
  finally { trashBusy.value = false }
}

async function restoreTrashSelected() {
  const ids = [...trashSelected.value]
  if (!ids.length) return
  trashBusy.value = true; trashResult.value = null; adminError.value = null
  try {
    const res = await api.adminRestoreTrash(ids)
    trashResult.value = `Restored ${res.restored} image${res.restored !== 1 ? 's' : ''} — reassign via Photo Detail.`
    trashSelected.value = new Set()
    await loadTrash()
  } catch (e) { adminError.value = `Restore failed: ${errMsg(e)}` }
  finally { trashBusy.value = false }
}

// ── Deleted cards ─────────────────────────────────────────────────────────────
type DeletedCard = { id: string; name: string; deletedAt: string }
const deletedCards       = ref<DeletedCard[]>([])
const deletedCardsBusy   = ref(false)
const deletedCardsResult = ref<string | null>(null)
const confirmPurgeId     = ref<string | null>(null)

async function loadDeletedCards() {
  deletedCardsBusy.value = true
  try {
    const res = await api.adminListDeletedCards()
    deletedCards.value = res.cards
  } catch (_e) { /* non-fatal */ }
  finally { deletedCardsBusy.value = false }
}

async function restoreCard(id: string) {
  deletedCardsBusy.value = true
  deletedCardsResult.value = null
  adminError.value = null
  try {
    await api.adminRestoreCard(id)
    deletedCardsResult.value = 'Card restored.'
    await cards.load()
    await loadDeletedCards()
  } catch (e) { adminError.value = `Restore failed: ${errMsg(e)}` }
  finally { deletedCardsBusy.value = false }
}

async function purgeCard(id: string, name: string) {
  deletedCardsBusy.value = true
  deletedCardsResult.value = null
  adminError.value = null
  confirmPurgeId.value = null
  try {
    await api.adminPurgeCard(id)
    deletedCardsResult.value = `"${name}" permanently deleted.`
    await loadDeletedCards()
  } catch (e) { adminError.value = `Delete failed: ${errMsg(e)}` }
  finally { deletedCardsBusy.value = false }
}

function onTabTools() {
  tab.value = 'tools'
  orphanScan.value = null
  orphanResult.value = null
  exportResult.value = null
  reloadResult.value = null
  trashResult.value = null
  adminError.value = null
  loadAdminStats()
  loadTrash()
  loadDeletedCards()
}

// ── Export: YAML + legacy card repair ────────────────────────────────────────
const CATEGORY_ALIASES: Record<string, string> = {
  'Platform & Handling':     'Platform and Handling',
  'Tires & Rims':            'Tires and Wheels',
  'Bodykits and Conversion': 'Body Kits and Conversions',
  'Body Kits and Conversion':'Body Kits and Conversions',
}

interface LegacyRow { name: string; description: string }
type AnyRow = AdjustmentRow | LegacyRow
function isLegacyRow(row: AnyRow): row is LegacyRow { return 'name' in row && !('tab' in row) }
function getRecipe(card: Card): ForzaRecipeSection | undefined {
  return card.sections.find((s): s is ForzaRecipeSection => s.type === 'forza_recipe')
}

interface MigrateStatus { card: Card; needsCategories: boolean; legacyCount: number }
const migrateStatus = computed<MigrateStatus[]>(() =>
  cards.cards.filter(c => !c.isLegend).map(c => {
    const r = getRecipe(c)
    if (!r) return { card: c, needsCategories: false, legacyCount: 0 }
    return {
      card: c,
      needsCategories: r.upgrades.some(u => !!CATEGORY_ALIASES[u.category]),
      legacyCount: (r.adjustments as AnyRow[]).filter(isLegacyRow).length,
    }
  })
)

const catBusy   = ref(false)
const catResult = ref<string | null>(null)

async function fixAllCategories() {
  catBusy.value = true; catResult.value = null
  let fixed = 0
  try {
    for (const { card, needsCategories } of migrateStatus.value) {
      if (!needsCategories) continue
      const storeCard = cards.byId(card.id)!
      const recipe = getRecipe(storeCard)!
      recipe.upgrades = recipe.upgrades.map(u => ({ ...u, category: CATEGORY_ALIASES[u.category] ?? u.category }))
      await cards.save(card.id)
      fixed++
    }
    catResult.value = fixed ? `Fixed ${fixed} card${fixed !== 1 ? 's' : ''}.` : 'Nothing to fix.'
  } catch (e) { catResult.value = `Error: ${errMsg(e)}` }
  finally { catBusy.value = false }
}

const adjCardId  = ref<string | null>(null)
const adjRowIdx  = ref(0)
const adjResults = ref<Map<number, AdjustmentRow[] | 'skip'>>(new Map())
const adjBusy    = ref(false)
const adjResult  = ref<string | null>(null)

const TABS = ['tires', 'alignment', 'arb', 'springs', 'damping', 'aero', 'brakes', 'differential', 'gearing']
const TAB_DEFAULTS: Record<string, Pick<AdjustmentRow, 'unit' | 'min' | 'max' | 'stock' | 'step'>> = {
  tires:        { unit: 'psi', min: 20,  max: 50,   stock: 32,  step: 0.5 },
  alignment:    { unit: '°',   min: -5,  max: 5,    stock: 0,   step: 0.1 },
  arb:          { unit: '',    min: 1,   max: 65,   stock: 5,   step: 0.5 },
  springs:      { unit: '',    min: 100, max: 2000,  stock: 500, step: 10  },
  damping:      { unit: '',    min: 1,   max: 20,   stock: 5,   step: 0.1 },
  aero:         { unit: '',    min: 0,   max: 500,  stock: 0,   step: 1   },
  brakes:       { unit: '%',   min: 0,   max: 200,  stock: 50,  step: 1   },
  differential: { unit: '%',   min: 0,   max: 100,  stock: 50,  step: 1   },
  gearing:      { unit: '',    min: 0,   max: 10,   stock: 3,   step: 0.01},
}

const adjDraft = ref<AdjustmentRow>({ tab: 'arb', group: '', key: '', label: '', unit: '', min: 0, max: 0, stock: 0, value: 0, step: 1 })
const adjLegacyRows = computed<LegacyRow[]>(() => {
  if (!adjCardId.value) return []
  const card = cards.byId(adjCardId.value)
  const r = card ? getRecipe(card) : undefined
  return r ? (r.adjustments as AnyRow[]).filter(isLegacyRow) : []
})
const adjCurrentRow   = computed(() => adjLegacyRows.value[adjRowIdx.value])
const adjCurrentSaved = computed<AdjustmentRow[]>(() => { const r = adjResults.value.get(adjRowIdx.value); return Array.isArray(r) ? r : [] })
const adjAllHandled   = computed(() => adjResults.value.size >= adjLegacyRows.value.length)

function openAdjCard(cardId: string) { adjCardId.value = cardId; adjRowIdx.value = 0; adjResults.value = new Map(); adjResult.value = null; applyTabDefaults() }
function applyTabDefaults() { const d = TAB_DEFAULTS[adjDraft.value.tab]; if (d) adjDraft.value = { ...adjDraft.value, ...d, value: d.stock } }
function onAdjTabChange() { adjDraft.value.group = ''; adjDraft.value.label = ''; applyTabDefaults() }
function adjAutoKey() { return adjDraft.value.tab + adjDraft.value.group.replace(/\s+/g, '') + adjDraft.value.label.replace(/\s+/g, '') }
function saveRow() { adjResults.value.set(adjRowIdx.value, [...adjCurrentSaved.value, { ...adjDraft.value, key: adjAutoKey() }]); adjDraft.value.label = '' }
function nextRow() { if (adjRowIdx.value < adjLegacyRows.value.length - 1) { adjRowIdx.value++; applyTabDefaults() } }
function skipRow() { adjResults.value.set(adjRowIdx.value, 'skip'); if (adjRowIdx.value < adjLegacyRows.value.length - 1) { adjRowIdx.value++; applyTabDefaults() } }

async function commitAdjMigration() {
  if (!adjCardId.value) return
  adjBusy.value = true; adjResult.value = null
  try {
    const storeCard = cards.byId(adjCardId.value)!
    const recipe = getRecipe(storeCard)!
    const rows = recipe.adjustments as AnyRow[]
    const newAdj: AdjustmentRow[] = []
    let legacyIdx = 0
    for (const row of rows) {
      if (isLegacyRow(row)) { const result = adjResults.value.get(legacyIdx); if (Array.isArray(result)) newAdj.push(...result); legacyIdx++ }
      else newAdj.push(row)
    }
    recipe.adjustments = newAdj
    await cards.save(adjCardId.value)
    const saved = [...adjResults.value.values()].reduce<number>((n, v) => n + (Array.isArray(v) ? v.length : 0), 0)
    const skipped = [...adjResults.value.values()].filter(v => v === 'skip').length
    adjResult.value = `Saved ${saved} row${saved !== 1 ? 's' : ''}${skipped ? `, skipped ${skipped}` : ''}.`
    adjCardId.value = null
  } catch (e) { adjResult.value = `Error: ${errMsg(e)}` }
  finally { adjBusy.value = false }
}

function onTabExport() { tab.value = 'export'; adjCardId.value = null; catResult.value = null; adjResult.value = null }

// YAML
function downloadCardYaml(card: Card) {
  const text = cardToYaml(card)
  const blob = new Blob([text], { type: 'text/yaml' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${card.name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '') || 'card'}.yaml`
  a.click()
  URL.revokeObjectURL(url)
}

const importError   = ref<string | null>(null)
const importPreview = ref<Omit<Card, 'id' | 'catalogNumber'> | null>(null)
const importBusy    = ref(false)
const importResult  = ref<string | null>(null)
const importFileRef = ref<HTMLInputElement | null>(null)

function onImportFile(e: Event) {
  importError.value = null; importPreview.value = null; importResult.value = null
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  file.text().then(text => {
    const result = yamlToCard(text)
    if (!result.ok) importError.value = result.error
    else importPreview.value = result.card
  })
}

async function confirmImport() {
  if (!importPreview.value) return
  importBusy.value = true; importResult.value = null
  try {
    const maxNum = Math.max(0, ...cards.cards.map(c => c.catalogNumber))
    const newCard: Card = { ...importPreview.value, id: crypto.randomUUID(), catalogNumber: maxNum + 1 }
    await api.createCard(newCard)
    await cards.load()
    importResult.value = `Imported "${newCard.name}" as card #${newCard.catalogNumber}.`
    importPreview.value = null
    if (importFileRef.value) importFileRef.value.value = ''
  } catch (e) { importResult.value = `Error: ${errMsg(e)}` }
  finally { importBusy.value = false }
}

function cancelImport() { importPreview.value = null; importError.value = null; if (importFileRef.value) importFileRef.value.value = '' }
</script>

<template>
  <div v-if="modal.adminPanelOpen" class="image-picker open" @click.self="close()">
    <div class="image-picker-panel admin-panel-modal" @wheel.stop>
      <div class="image-picker-head">
        <span>Admin</span>
        <button class="image-picker-close" aria-label="Close" @click="close()">×</button>
      </div>

      <div class="settings-tabs">
        <button :class="{ active: tab === 'tools' }" @click="onTabTools">Tools</button>
        <button :class="{ active: tab === 'export' }" @click="onTabExport">Export Card</button>
      </div>

      <!-- Tools -->
      <div v-if="tab === 'tools'" class="ap-sections">
        <p v-if="adminError" class="settings-error">{{ adminError }}</p>

        <div class="admin-section">
          <div class="admin-section-head">Image Tools</div>
          <div class="admin-row">
            <button class="admin-btn" @click="modal.openImageMigration(); close()">
              Image Migration
              <span v-if="failedAssess.length" class="admin-badge">{{ failedAssess.length }}</span>
            </button>
            <button class="admin-btn" :disabled="repairBusy" @click="repairFigurePaths">
              {{ repairBusy ? 'Repairing…' : 'Repair Figure Paths' }}
            </button>
          </div>
          <p v-if="repairResult" class="admin-ok">{{ repairResult }}</p>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">System Stats</div>
          <div v-if="adminStatsBusy" class="admin-muted">Loading…</div>
          <table v-else-if="adminStats" class="admin-stats-table">
            <tr><td>Cards</td><td>{{ adminStats.cardCount }}</td></tr>
            <tr><td>Images (in DB)</td><td>{{ adminStats.imageCount }}</td></tr>
            <tr><td>Files on disk</td><td>{{ adminStats.fileCount }}</td></tr>
            <tr><td>Uploads size</td><td>{{ formatBytes(adminStats.uploadsDirBytes) }}</td></tr>
            <tr><td>Database size</td><td>{{ formatBytes(adminStats.dbBytes) }}</td></tr>
          </table>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">Orphan Files</div>
          <p class="admin-muted">Files in uploads with no images table reference.</p>
          <div class="admin-row">
            <button class="admin-btn" :disabled="orphanBusy" @click="scanOrphans">
              {{ orphanBusy && !orphanScan ? 'Scanning…' : 'Scan' }}
            </button>
            <button v-if="orphanScan && orphanScan.count > 0" class="admin-btn" :disabled="orphanBusy" @click="deleteOrphans">
              {{ orphanBusy ? 'Sweeping…' : `Move ${orphanScan.count} to Trash` }}
            </button>
          </div>
          <p v-if="orphanScan && orphanScan.count === 0" class="admin-ok">No orphans found.</p>
          <p v-if="orphanResult" class="admin-ok">{{ orphanResult }}</p>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">
            Trash
            <span v-if="trashEntries.length" class="admin-badge">{{ trashEntries.length }}</span>
          </div>
          <div v-if="trashBusy && !trashEntries.length" class="admin-muted">Loading…</div>
          <template v-else>
            <p v-if="!trashEntries.length" class="admin-muted">Trash is empty.</p>
            <template v-else>
              <div class="admin-row" style="justify-content: space-between; align-items: center;">
                <span class="admin-muted">{{ trashEntries.length }} file{{ trashEntries.length !== 1 ? 's' : '' }} · {{ formatBytes(trashTotalBytes) }}</span>
                <button class="admin-btn admin-btn-link" @click="trashExpanded = !trashExpanded">
                  {{ trashExpanded ? 'Collapse' : 'View' }}
                </button>
              </div>
              <div v-if="trashExpanded" class="trash-list">
                <div class="trash-select-row">
                  <label class="trash-check-label">
                    <input type="checkbox" :checked="trashSelected.size === trashEntries.filter(e => e.id !== null).length && trashEntries.some(e => e.id !== null)" @change="toggleSelectAll" />
                    <span>Select all</span>
                  </label>
                </div>
                <div v-for="entry in trashEntries" :key="entry.trashFilename" class="trash-item" :class="{ 'trash-item--selected': entry.id !== null && trashSelected.has(entry.id) }">
                  <input v-if="entry.id !== null" type="checkbox" :checked="trashSelected.has(entry.id)" class="trash-item-check" @change="toggleTrashSelect(entry.id)" />
                  <div v-else class="trash-item-check-placeholder" />
                  <div class="trash-item-info">
                    <span class="trash-item-name" :title="entry.originalPath ?? entry.trashFilename">
                      {{ entry.originalPath?.split('/').pop() ?? entry.trashFilename }}
                    </span>
                    <span class="trash-item-meta">
                      <span class="trash-reason" :class="`trash-reason--${entry.reason}`">{{ entry.reason }}</span>
                      <span v-if="entry.cardId" class="trash-card-id">{{ entry.cardId }}</span>
                      <span>{{ formatBytes(entry.bytes) }}</span>
                    </span>
                  </div>
                </div>
              </div>
              <div v-if="trashExpanded" class="admin-row" style="margin-top:4px;">
                <button class="admin-btn" :disabled="trashBusy || trashSelected.size === 0" @click="restoreTrashSelected">
                  {{ trashBusy ? 'Restoring…' : `Restore (${trashSelected.size})` }}
                </button>
                <button class="admin-btn admin-btn-red" :disabled="trashBusy || trashSelected.size === 0" @click="deleteTrashSelected">
                  {{ trashBusy ? 'Deleting…' : `Delete (${trashSelected.size})` }}
                </button>
                <button class="admin-btn admin-btn-red" :disabled="trashBusy" @click="deleteAllTrash">
                  {{ trashBusy ? 'Deleting…' : 'Delete All' }}
                </button>
              </div>
            </template>
            <p v-if="trashResult" class="admin-ok">{{ trashResult }}</p>
          </template>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">
            Deleted Cards
            <span v-if="deletedCards.length" class="admin-badge">{{ deletedCards.length }}</span>
          </div>
          <div v-if="deletedCardsBusy && !deletedCards.length" class="admin-muted">Loading…</div>
          <template v-else>
            <p v-if="!deletedCards.length" class="admin-muted">No archived cards.</p>
            <div v-else class="deleted-card-list">
              <div v-for="c in deletedCards" :key="c.id" class="deleted-card-row">
                <div class="deleted-card-info">
                  <span class="deleted-card-name">{{ c.name }}</span>
                  <span class="deleted-card-date">{{ c.deletedAt.slice(0, 10) }}</span>
                </div>
                <div v-if="confirmPurgeId !== c.id" class="admin-row">
                  <button class="admin-btn mig-btn-sm" :disabled="deletedCardsBusy" @click="restoreCard(c.id)">Restore</button>
                  <button class="admin-btn admin-btn-red mig-btn-sm" :disabled="deletedCardsBusy" @click="confirmPurgeId = c.id">Delete</button>
                </div>
                <div v-else class="admin-row">
                  <span class="deleted-purge-label">Sure?</span>
                  <button class="admin-btn admin-btn-red mig-btn-sm" :disabled="deletedCardsBusy" @click="purgeCard(c.id, c.name)">Yes, delete</button>
                  <button class="admin-btn mig-btn-sm" @click="confirmPurgeId = null">Cancel</button>
                </div>
              </div>
            </div>
            <p v-if="deletedCardsResult" class="admin-ok">{{ deletedCardsResult }}</p>
          </template>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">Seed</div>
          <p class="admin-muted">Export writes current DB to the seed file. Reload applies the seed file to the live DB.</p>
          <div class="admin-row">
            <button class="admin-btn" :disabled="exportBusy" @click="exportSeed">
              {{ exportBusy ? 'Exporting…' : 'Export Seed' }}
            </button>
            <button class="admin-btn" :disabled="reloadBusy" @click="reloadSeed">
              {{ reloadBusy ? 'Reloading…' : 'Reload from Seed' }}
            </button>
          </div>
          <p v-if="exportResult" class="admin-ok">{{ exportResult }}</p>
          <p v-if="reloadResult" class="admin-ok">{{ reloadResult }}</p>
        </div>
      </div>

      <!-- Export Card -->
      <div v-if="tab === 'export'" class="ap-sections">

        <div class="admin-section">
          <div class="admin-section-head">Download YAML</div>
          <p class="admin-muted">Download a human-readable YAML file for any card. Images are not included.</p>
          <div class="mig-card-list">
            <div v-for="s in migrateStatus" :key="s.card.id" class="mig-card-row">
              <span class="mig-card-name">{{ s.card.name }}</span>
              <button class="admin-btn mig-btn-sm" @click="downloadCardYaml(s.card)">Download ↓</button>
            </div>
          </div>
        </div>

        <div class="admin-section">
          <div class="admin-section-head">Import Card from YAML</div>
          <p class="admin-muted">Create a new card from a YAML export. Images must be added manually after import.</p>
          <div class="admin-row">
            <label class="admin-btn mig-file-label">
              Choose File
              <input ref="importFileRef" type="file" accept=".yaml,.yml" class="mig-file-input" @change="onImportFile" />
            </label>
          </div>
          <p v-if="importError" class="admin-err">{{ importError }}</p>
          <div v-if="importPreview" class="mig-preview">
            <div class="mig-preview-name">{{ importPreview.name }}</div>
            <div v-if="importPreview.subtitle" class="mig-preview-sub">{{ importPreview.subtitle }}</div>
            <div class="mig-preview-meta">
              {{ importPreview.sections.length }} section{{ importPreview.sections.length !== 1 ? 's' : '' }}
              <template v-if="importPreview.collections.length"> · {{ importPreview.collections.join(', ') }}</template>
            </div>
            <div class="admin-row" style="margin-top: 10px;">
              <button class="admin-btn" :disabled="importBusy" @click="confirmImport">{{ importBusy ? 'Importing…' : 'Import as New Card' }}</button>
              <button class="admin-btn" @click="cancelImport">Cancel</button>
            </div>
          </div>
          <p v-if="importResult" class="admin-ok">{{ importResult }}</p>
        </div>

        <!-- Legacy repair — buried at bottom since mostly done -->
        <div class="admin-section legacy-section">
          <div class="admin-section-head">Legacy Repair</div>
          <p class="admin-muted">One-time tools for fixing data from the original import. Mostly done.</p>

          <div class="admin-section" style="margin-top:8px;">
            <div class="admin-section-head" style="font-size:9px;">Upgrade Category Names</div>
            <div class="mig-card-list">
              <div v-for="s in migrateStatus" :key="s.card.id" class="mig-card-row">
                <span class="mig-card-name">{{ s.card.name }}</span>
                <span v-if="s.needsCategories" class="mig-badge mig-badge--warn">⚠ needs fix</span>
                <span v-else class="mig-badge mig-badge--ok">✓</span>
              </div>
              <div v-if="migrateStatus.every(s => !s.needsCategories)" class="admin-muted">All canonical.</div>
            </div>
            <div v-if="migrateStatus.some(s => s.needsCategories)" class="admin-row">
              <button class="admin-btn" :disabled="catBusy" @click="fixAllCategories">{{ catBusy ? 'Fixing…' : 'Fix All' }}</button>
            </div>
            <p v-if="catResult" class="admin-ok">{{ catResult }}</p>
          </div>

          <div class="admin-section" style="margin-top:12px;">
            <div class="admin-section-head" style="font-size:9px;">Adjustment Rows</div>
            <template v-if="!adjCardId">
              <div class="mig-card-list">
                <div v-for="s in migrateStatus.filter(s => s.legacyCount > 0)" :key="s.card.id" class="mig-card-row">
                  <span class="mig-card-name">{{ s.card.name }}</span>
                  <span class="mig-badge mig-badge--warn">⚠ {{ s.legacyCount }} row{{ s.legacyCount !== 1 ? 's' : '' }}</span>
                  <button class="admin-btn mig-btn-sm" @click="openAdjCard(s.card.id)">Migrate →</button>
                </div>
                <div v-if="migrateStatus.every(s => s.legacyCount === 0)" class="admin-muted">All rows structured.</div>
              </div>
              <p v-if="adjResult" class="admin-ok">{{ adjResult }}</p>
            </template>
            <template v-else>
              <div class="mig-form-header">
                <span class="mig-form-title">{{ cards.byId(adjCardId)?.name }}</span>
                <span class="mig-form-progress">Row {{ adjRowIdx + 1 }} / {{ adjLegacyRows.length }}</span>
                <button class="admin-btn mig-btn-sm" @click="adjCardId = null">← Back</button>
              </div>
              <div v-if="adjCurrentRow" class="mig-source-row">
                <div class="mig-source-label">{{ adjCurrentRow.name }}</div>
                <div class="mig-source-desc">{{ adjCurrentRow.description }}</div>
              </div>
              <div v-if="adjResults.has(adjRowIdx)" class="mig-handled">
                <span v-if="adjResults.get(adjRowIdx) === 'skip'" class="mig-badge mig-badge--skip">Skipped</span>
                <template v-else>
                  <span v-for="(r, i) in adjCurrentSaved" :key="i" class="mig-badge mig-badge--ok">{{ r.group }} {{ r.label }}</span>
                </template>
              </div>
              <div v-if="!adjAllHandled" class="mig-form">
                <div class="mig-field-row">
                  <label class="mig-label">Tab</label>
                  <select v-model="adjDraft.tab" class="mig-select" @change="onAdjTabChange">
                    <option v-for="t in TABS" :key="t" :value="t">{{ t }}</option>
                  </select>
                </div>
                <div class="mig-field-row"><label class="mig-label">Group</label><input v-model="adjDraft.group" class="mig-input" placeholder="e.g. Front Anti-Roll Bar" /></div>
                <div class="mig-field-row"><label class="mig-label">Label</label><input v-model="adjDraft.label" class="mig-input" placeholder="e.g. Front" /></div>
                <div class="mig-field-row"><label class="mig-label">Unit</label><input v-model="adjDraft.unit" class="mig-input mig-input--short" placeholder="° or psi or blank" /></div>
                <div class="mig-nums-row">
                  <div class="mig-num"><label class="mig-label">Min</label><input v-model.number="adjDraft.min" type="number" class="mig-input mig-input--num" /></div>
                  <div class="mig-num"><label class="mig-label">Max</label><input v-model.number="adjDraft.max" type="number" class="mig-input mig-input--num" /></div>
                  <div class="mig-num"><label class="mig-label">Stock</label><input v-model.number="adjDraft.stock" type="number" class="mig-input mig-input--num" /></div>
                  <div class="mig-num"><label class="mig-label">Value</label><input v-model.number="adjDraft.value" type="number" class="mig-input mig-input--num" /></div>
                  <div class="mig-num"><label class="mig-label">Step</label><input v-model.number="adjDraft.step" type="number" class="mig-input mig-input--num" /></div>
                </div>
                <div class="admin-row">
                  <button class="admin-btn" @click="saveRow">Save Row</button>
                  <button class="admin-btn" :disabled="adjCurrentSaved.length === 0" @click="nextRow">Next →</button>
                  <button class="admin-btn" @click="skipRow">Skip</button>
                </div>
              </div>
              <div v-if="adjAllHandled" class="mig-commit">
                <p class="admin-muted">All rows handled. Commit to save the card.</p>
                <button class="admin-btn" :disabled="adjBusy" @click="commitAdjMigration">{{ adjBusy ? 'Saving…' : 'Commit Migration' }}</button>
                <p v-if="adjResult" class="admin-ok">{{ adjResult }}</p>
              </div>
            </template>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.admin-panel-modal {
  width: 480px;
  max-width: 96vw;
  max-height: 88vh;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.settings-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--panel-edge);
}
.settings-tabs button {
  flex: 1;
  padding: 7px 8px;
  border: none;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  background: none;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.settings-tabs button:hover { color: var(--fg); }
.settings-tabs button.active { color: var(--accent); border-bottom-color: var(--accent); }

.settings-error { color: var(--danger-bright); font-size: 13px; margin: 0; }
.settings-ok    { color: var(--accent);         font-size: 11px; margin: 0; }

.ap-sections { display: flex; flex-direction: column; gap: 20px; padding: 4px 2px 2px; }
.admin-section { display: flex; flex-direction: column; gap: 8px; }
.admin-section-head {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent);
  padding-bottom: 4px;
  border-bottom: 1px solid var(--panel-edge);
}
.admin-stats-table { width: 100%; border-collapse: collapse; font-family: 'JetBrains Mono', monospace; font-size: 11px; }
.admin-stats-table td { padding: 3px 0; }
.admin-stats-table td:first-child { color: var(--muted); }
.admin-stats-table td:last-child { text-align: right; color: var(--fg); }
.admin-muted { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--muted); margin: 0; }
.admin-ok    { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--accent); margin: 0; }
.admin-row   { display: flex; gap: 8px; flex-wrap: wrap; }
.admin-btn {
  padding: 7px 14px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.admin-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.admin-btn:disabled { opacity: 0.5; cursor: default; }
.admin-btn-red { border-color: var(--danger); background: color-mix(in srgb, var(--danger) 35%, transparent); color: var(--danger-bright); }
.admin-btn-red:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 60%, transparent); border-color: var(--danger-bright); box-shadow: 0 0 12px color-mix(in srgb, var(--danger-bright) 50%, transparent); }
.admin-btn-link { background: none; border-color: transparent; padding: 2px 6px; font-size: 10px; color: var(--accent); }
.admin-btn-link:hover:not(:disabled) { border-color: var(--accent); background: none; }
.admin-badge { display: inline-block; background: var(--highlight); color: #fff; font-size: 9px; border-radius: 100px; padding: 1px 6px; margin-left: 6px; vertical-align: middle; }

.legacy-section {
  padding-top: 16px;
  border-top: 1px dashed var(--panel-edge);
  opacity: 0.7;
}
.legacy-section:hover { opacity: 1; }

.mig-card-list { display: flex; flex-direction: column; gap: 4px; }
.mig-card-row { display: flex; align-items: center; gap: 8px; font-family: 'JetBrains Mono', monospace; font-size: 11px; }
.mig-card-name { flex: 1; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mig-badge { padding: 1px 6px; border-radius: 4px; font-size: 10px; flex-shrink: 0; }
.mig-badge--ok   { background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent); }
.mig-badge--warn { background: color-mix(in srgb, var(--highlight) 15%, transparent); color: var(--highlight); }
.mig-badge--skip { background: color-mix(in srgb, var(--muted) 15%, transparent); color: var(--muted); }
.mig-btn-sm { padding: 3px 10px; font-size: 10px; flex-shrink: 0; }
.mig-form-header { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
.mig-form-title { flex: 1; font-family: 'JetBrains Mono', monospace; font-size: 12px; font-weight: 600; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mig-form-progress { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--muted); flex-shrink: 0; }
.mig-source-row { border-left: 2px solid var(--highlight); padding: 6px 10px; margin-bottom: 10px; background: color-mix(in srgb, var(--highlight) 6%, transparent); border-radius: 0 4px 4px 0; }
.mig-source-label { font-family: 'JetBrains Mono', monospace; font-size: 11px; font-weight: 600; color: var(--fg); }
.mig-source-desc { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--muted-light); margin-top: 2px; }
.mig-handled { margin-bottom: 8px; }
.mig-form { display: flex; flex-direction: column; gap: 8px; }
.mig-field-row { display: flex; align-items: center; gap: 8px; }
.mig-label { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; width: 46px; flex-shrink: 0; }
.mig-input { flex: 1; background: var(--panel-well); border: 1px solid var(--panel-edge); border-radius: 4px; color: var(--fg); font-family: 'JetBrains Mono', monospace; font-size: 11px; padding: 5px 8px; }
.mig-input:focus { outline: none; border-color: var(--accent); }
.mig-input--short { max-width: 90px; }
.mig-input--num { width: 72px; flex: none; }
.mig-select { flex: 1; background: var(--panel-well); border: 1px solid var(--panel-edge); border-radius: 4px; color: var(--fg); font-family: 'JetBrains Mono', monospace; font-size: 11px; padding: 5px 8px; }
.mig-select:focus { outline: none; border-color: var(--accent); }
.mig-nums-row { display: flex; gap: 8px; flex-wrap: wrap; }
.mig-num { display: flex; flex-direction: column; gap: 3px; }
.mig-num .mig-label { width: auto; }
.mig-commit { display: flex; flex-direction: column; gap: 8px; }
.admin-err { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--danger-bright, #e05050); margin-top: 6px; }
.mig-file-label { position: relative; overflow: hidden; cursor: pointer; display: inline-block; }
.mig-file-input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
.mig-preview { margin-top: 8px; border: 1px solid var(--panel-edge); border-radius: 5px; padding: 10px 12px; background: color-mix(in srgb, var(--panel) 60%, transparent); }
.mig-preview-name { font: 700 13px/1.3 'Oswald', sans-serif; letter-spacing: 0.04em; color: var(--fg); }
.mig-preview-sub  { font-size: 11px; color: var(--muted); margin-top: 2px; }
.mig-preview-meta { font: 11px/1.3 'JetBrains Mono', monospace; color: var(--muted); opacity: 0.6; margin-top: 6px; }

.trash-list { display: flex; flex-direction: column; gap: 2px; max-height: 220px; overflow-y: auto; overscroll-behavior: contain; border: 1px solid var(--panel-edge); border-radius: 4px; padding: 4px; }
.trash-select-row { padding: 3px 4px 5px; border-bottom: 1px solid var(--panel-edge); margin-bottom: 2px; }
.trash-check-label { display: flex; align-items: center; gap: 6px; font: 10px/1 'JetBrains Mono', monospace; color: var(--muted); cursor: pointer; text-transform: uppercase; letter-spacing: 0.05em; }
.trash-item { display: flex; align-items: flex-start; gap: 6px; padding: 5px 4px; border-radius: 3px; transition: background 0.1s; }
.trash-item--selected { background: color-mix(in srgb, var(--accent) 8%, transparent); }
.trash-item-check { flex-shrink: 0; margin-top: 2px; cursor: pointer; }
.trash-item-check-placeholder { width: 13px; flex-shrink: 0; }
.trash-item-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.trash-item-name { font: 11px/1.3 'JetBrains Mono', monospace; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 380px; }
.trash-item-meta { display: flex; gap: 6px; align-items: center; font: 10px/1 'JetBrains Mono', monospace; color: var(--muted); }
.trash-reason { padding: 1px 5px; border-radius: 3px; font-size: 9px; text-transform: uppercase; letter-spacing: 0.04em; }
.trash-reason--orphan      { background: color-mix(in srgb, var(--muted) 15%, transparent);     color: var(--muted); }
.trash-reason--user_delete { background: color-mix(in srgb, var(--highlight) 15%, transparent); color: var(--highlight); }
.trash-reason--unknown     { background: color-mix(in srgb, var(--danger) 12%, transparent);    color: var(--danger-bright); }
.trash-card-id { opacity: 0.6; font-size: 9px; }

.deleted-card-list { display: flex; flex-direction: column; gap: 6px; }
.deleted-card-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 6px 0; border-bottom: 1px solid var(--panel-edge); }
.deleted-card-row:last-child { border-bottom: none; }
.deleted-card-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.deleted-card-name { font: 12px/1.3 'JetBrains Mono', monospace; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.deleted-card-date { font: 10px/1 'JetBrains Mono', monospace; color: var(--muted); }
.deleted-purge-label { font: 10px/1 'JetBrains Mono', monospace; color: var(--danger-bright); text-transform: uppercase; letter-spacing: 0.05em; align-self: center; }
</style>
