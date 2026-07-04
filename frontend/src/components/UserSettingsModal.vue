<script setup lang="ts">
import { ref, computed } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useAuthStore } from '../stores/auth'
import { useCardsStore } from '../stores/cards'
import { api } from '../api'
import type { Card, ForzaRecipeSection, AdjustmentRow } from '../types'

const ui = useUiStore()
const modal = useModalStore()
const auth = useAuthStore()

type Tab = 'password' | 'create' | 'admin' | 'migrate'
const tab = ref<Tab>('password')

// Change password
const currentPw  = ref('')
const newPw      = ref('')
const confirmPw  = ref('')
const pwError    = ref('')
const pwSuccess  = ref('')
const pwBusy     = ref(false)

async function submitChangePassword() {
  pwError.value = ''
  pwSuccess.value = ''
  if (newPw.value !== confirmPw.value) { pwError.value = 'New passwords do not match.'; return }
  if (newPw.value.length < 8) { pwError.value = 'Password must be at least 8 characters.'; return }
  pwBusy.value = true
  try {
    await api.changePassword(currentPw.value, newPw.value)
    pwSuccess.value = 'Password updated.'
    currentPw.value = ''; newPw.value = ''; confirmPw.value = ''
  } catch (e: any) {
    pwError.value = e.message?.includes('incorrect') ? 'Current password is incorrect.' : 'Failed to update password.'
  } finally {
    pwBusy.value = false
  }
}

// Create user
const newUsername = ref('')
const newUserPw   = ref('')
const newUserConfirm = ref('')
const userError   = ref('')
const userSuccess = ref('')
const userBusy    = ref(false)

async function submitCreateUser() {
  userError.value = ''
  userSuccess.value = ''
  if (!newUsername.value.trim()) { userError.value = 'Username is required.'; return }
  if (newUserPw.value !== newUserConfirm.value) { userError.value = 'Passwords do not match.'; return }
  if (newUserPw.value.length < 8) { userError.value = 'Password must be at least 8 characters.'; return }
  userBusy.value = true
  try {
    const res = await api.createUser(newUsername.value.trim(), newUserPw.value)
    userSuccess.value = `User '${res.username}' created.`
    newUsername.value = ''; newUserPw.value = ''; newUserConfirm.value = ''
  } catch (e: any) {
    userError.value = e.message?.includes('already exists') ? 'That username is already taken.' : 'Failed to create user.'
  } finally {
    userBusy.value = false
  }
}

function close() {
  modal.closeSettings()
  pwError.value = ''; pwSuccess.value = ''
  userError.value = ''; userSuccess.value = ''
  currentPw.value = ''; newPw.value = ''; confirmPw.value = ''
  newUsername.value = ''; newUserPw.value = ''; newUserConfirm.value = ''
}

function logout() {
  if (ui.isEditing) ui.toggleEdit()
  auth.logout()
  modal.closeSettings()
}

// Suggestions
type Suggestion = { id: number; cardId: string; title: string; credit: string | null; adjustments: object[]; submittedAt: string; ip: string; reviewed: boolean }
const suggestions = ref<Suggestion[]>([])
const suggestionsBusy = ref(false)
const suggestionsError = ref<string | null>(null)

async function loadSuggestions() {
  suggestionsBusy.value = true
  suggestionsError.value = null
  try { suggestions.value = await api.adminListSuggestions() }
  catch (e: any) { suggestionsError.value = `Failed: ${e.message}` }
  finally { suggestionsBusy.value = false }
}

async function dismissSuggestion(id: number) {
  try {
    await api.adminDismissSuggestion(id)
    suggestions.value = suggestions.value.filter(s => s.id !== id)
  } catch (e: any) { suggestionsError.value = `Failed: ${e.message}` }
}

// Admin
type AdminStats = { cardCount: number; imageCount: number; fileCount: number; uploadsDirBytes: number; dbBytes: number }
const adminStats = ref<AdminStats | null>(null)
const adminStatsBusy = ref(false)
const adminError = ref<string | null>(null)
const orphanScan = ref<{ count: number; paths: string[] } | null>(null)
const orphanBusy = ref(false)
const orphanResult = ref<string | null>(null)
const exportBusy = ref(false)
const exportResult = ref<string | null>(null)
const reloadBusy = ref(false)
const reloadResult = ref<string | null>(null)

function formatBytes(b: number) {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
  return `${(b / 1024 / 1024).toFixed(1)} MB`
}

async function loadAdminStats() {
  adminStatsBusy.value = true
  adminError.value = null
  try { adminStats.value = await api.adminStats() }
  catch (e: any) { adminError.value = `Stats failed: ${e.message}` }
  finally { adminStatsBusy.value = false }
}

async function scanOrphans() {
  orphanBusy.value = true
  orphanResult.value = null
  adminError.value = null
  try { orphanScan.value = await api.adminScanOrphans() }
  catch (e: any) { adminError.value = `Scan failed: ${e.message}` }
  finally { orphanBusy.value = false }
}

async function deleteOrphans() {
  orphanBusy.value = true
  adminError.value = null
  try {
    const res = await api.adminDeleteOrphans()
    orphanResult.value = `Deleted ${res.deleted} file${res.deleted !== 1 ? 's' : ''}.`
    orphanScan.value = null
  }
  catch (e: any) { adminError.value = `Delete failed: ${e.message}` }
  finally { orphanBusy.value = false }
}

async function exportSeed() {
  exportBusy.value = true
  exportResult.value = null
  adminError.value = null
  try {
    const res = await api.adminExportSeed()
    exportResult.value = `Exported ${res.exported} cards to seed file.`
  }
  catch (e: any) { adminError.value = `Export failed: ${e.message}` }
  finally { exportBusy.value = false }
}

async function reloadSeed() {
  reloadBusy.value = true
  reloadResult.value = null
  adminError.value = null
  try {
    const res = await api.adminReloadSeed()
    reloadResult.value = `Reloaded ${res.upserted} cards${res.removed > 0 ? `, removed ${res.removed}` : ''}.`
  }
  catch (e: any) { adminError.value = `Reload failed: ${e.message}` }
  finally { reloadBusy.value = false }
}

function onTabAdmin() {
  tab.value = 'admin'
  orphanScan.value = null
  orphanResult.value = null
  exportResult.value = null
  reloadResult.value = null
  adminError.value = null
  loadAdminStats()
  loadSuggestions()
}

// ── Migration ────────────────────────────────────────────────────────────────

const cards = useCardsStore()

const CATEGORY_ALIASES: Record<string, string> = {
  'Platform & Handling':    'Platform and Handling',
  'Tires & Rims':           'Tires and Wheels',
  'Bodykits and Conversion':'Body Kits and Conversions',
  'Body Kits and Conversion':'Body Kits and Conversions',
}

interface LegacyRow { name: string; description: string }
type AnyRow = AdjustmentRow | LegacyRow

function isLegacyRow(row: AnyRow): row is LegacyRow {
  return 'name' in row && !('tab' in row)
}

function getRecipe(card: Card): ForzaRecipeSection | undefined {
  return card.sections.find((s): s is ForzaRecipeSection => s.type === 'forza_recipe')
}

interface MigrateStatus { card: Card; needsCategories: boolean; legacyCount: number }

const migrateStatus = computed<MigrateStatus[]>(() =>
  cards.cards
    .filter(c => !c.isLegend)
    .map(c => {
      const r = getRecipe(c)
      if (!r) return { card: c, needsCategories: false, legacyCount: 0 }
      return {
        card: c,
        needsCategories: r.upgrades.some(u => !!CATEGORY_ALIASES[u.category]),
        legacyCount: (r.adjustments as AnyRow[]).filter(isLegacyRow).length,
      }
    })
)

const catBusy = ref(false)
const catResult = ref<string | null>(null)

async function fixAllCategories() {
  catBusy.value = true
  catResult.value = null
  let fixed = 0
  try {
    for (const { card, needsCategories } of migrateStatus.value) {
      if (!needsCategories) continue
      const storeCard = cards.byId(card.id)!
      const recipe = getRecipe(storeCard)!
      recipe.upgrades = recipe.upgrades.map(u => ({
        ...u, category: CATEGORY_ALIASES[u.category] ?? u.category,
      }))
      await cards.save(card.id)
      fixed++
    }
    catResult.value = fixed ? `Fixed ${fixed} card${fixed !== 1 ? 's' : ''}.` : 'Nothing to fix.'
  } catch (e: any) {
    catResult.value = `Error: ${e.message}`
  } finally {
    catBusy.value = false
  }
}

// Adjustment row migration — row-by-row form
const adjCardId = ref<string | null>(null)
const adjRowIdx  = ref(0)
// Each legacy row maps to an array of structured rows (1+) or 'skip'
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

const adjDraft = ref<AdjustmentRow>({
  tab: 'arb', group: '', key: '', label: '', unit: '',
  min: 0, max: 0, stock: 0, value: 0, step: 1,
})

const adjLegacyRows = computed<LegacyRow[]>(() => {
  if (!adjCardId.value) return []
  const card = cards.byId(adjCardId.value)
  const r = card ? getRecipe(card) : undefined
  return r ? (r.adjustments as AnyRow[]).filter(isLegacyRow) : []
})

const adjCurrentRow = computed(() => adjLegacyRows.value[adjRowIdx.value])
const adjCurrentSaved = computed<AdjustmentRow[]>(() => {
  const r = adjResults.value.get(adjRowIdx.value)
  return Array.isArray(r) ? r : []
})
const adjAllHandled = computed(() => adjResults.value.size >= adjLegacyRows.value.length)

function openAdjCard(cardId: string) {
  adjCardId.value = cardId
  adjRowIdx.value = 0
  adjResults.value = new Map()
  adjResult.value = null
  applyTabDefaults()
}

function applyTabDefaults() {
  const d = TAB_DEFAULTS[adjDraft.value.tab]
  if (!d) return
  adjDraft.value = { ...adjDraft.value, ...d, value: d.stock }
}

function onAdjTabChange() {
  adjDraft.value.group = ''
  adjDraft.value.label = ''
  applyTabDefaults()
}

function adjAutoKey(): string {
  const t = adjDraft.value.tab
  const g = adjDraft.value.group.replace(/\s+/g, '')
  const l = adjDraft.value.label.replace(/\s+/g, '')
  return t + g + l
}

function saveRow() {
  const existing = adjCurrentSaved.value
  adjResults.value.set(adjRowIdx.value, [...existing, { ...adjDraft.value, key: adjAutoKey() }])
  // Reset just label so user can fill in the next sub-row (e.g. Rear after Front)
  adjDraft.value.label = ''
}

function nextRow() {
  if (adjRowIdx.value < adjLegacyRows.value.length - 1) {
    adjRowIdx.value++
    applyTabDefaults()
  }
}

function skipRow() {
  adjResults.value.set(adjRowIdx.value, 'skip')
  if (adjRowIdx.value < adjLegacyRows.value.length - 1) {
    adjRowIdx.value++
    applyTabDefaults()
  }
}

async function commitAdjMigration() {
  if (!adjCardId.value) return
  adjBusy.value = true
  adjResult.value = null
  try {
    const storeCard = cards.byId(adjCardId.value)!
    const recipe = getRecipe(storeCard)!
    const rows = recipe.adjustments as AnyRow[]
    const newAdj: AdjustmentRow[] = []
    let legacyIdx = 0
    for (const row of rows) {
      if (isLegacyRow(row)) {
        const result = adjResults.value.get(legacyIdx)
        if (Array.isArray(result)) newAdj.push(...result)
        legacyIdx++
      } else {
        newAdj.push(row)
      }
    }
    recipe.adjustments = newAdj
    await cards.save(adjCardId.value)
    const saved = [...adjResults.value.values()].reduce<number>((n, v) => n + (Array.isArray(v) ? v.length : 0), 0)
    const skipped = [...adjResults.value.values()].filter(v => v === 'skip').length
    adjResult.value = `Saved ${saved} row${saved !== 1 ? 's' : ''}${skipped ? `, skipped ${skipped}` : ''}.`
    adjCardId.value = null
  } catch (e: any) {
    adjResult.value = `Error: ${e.message}`
  } finally {
    adjBusy.value = false
  }
}

function onTabMigrate() {
  tab.value = 'migrate'
  adjCardId.value = null
  catResult.value = null
  adjResult.value = null
}
</script>

<template>
  <div v-if="modal.settingsOpen" class="image-picker open" @click.self="close()">
    <div class="image-picker-panel settings-panel" :class="{ 'settings-panel--wide': tab === 'migrate' }">
      <div class="image-picker-head">
        <span>Account — {{ auth.username }}</span>
        <button class="image-picker-close" aria-label="Close" @click="close()">×</button>
      </div>

      <div class="settings-tabs">
        <button :class="{ active: tab === 'password' }" @click="tab = 'password'">Password</button>
        <button :class="{ active: tab === 'create' }" @click="tab = 'create'">Users</button>
        <button :class="{ active: tab === 'admin' }" @click="onTabAdmin">Admin</button>
        <button :class="{ active: tab === 'migrate' }" @click="onTabMigrate">Migrate</button>
      </div>

      <!-- Change Password -->
      <form v-if="tab === 'password'" class="settings-form" @submit.prevent="submitChangePassword">
        <input v-model="currentPw" type="password" placeholder="Current password" autocomplete="current-password" />
        <input v-model="newPw"     type="password" placeholder="New password"     autocomplete="new-password" />
        <input v-model="confirmPw" type="password" placeholder="Confirm new password" autocomplete="new-password" />
        <p v-if="pwError"   class="settings-error">{{ pwError }}</p>
        <p v-if="pwSuccess" class="settings-ok">{{ pwSuccess }}</p>
        <button type="submit" :disabled="pwBusy">{{ pwBusy ? 'Saving…' : 'Update Password' }}</button>
      </form>

      <!-- Create User -->
      <form v-if="tab === 'create'" class="settings-form" @submit.prevent="submitCreateUser">
        <input v-model="newUsername"    type="text"     placeholder="Username"         autocomplete="off" />
        <input v-model="newUserPw"      type="password" placeholder="Password"         autocomplete="new-password" />
        <input v-model="newUserConfirm" type="password" placeholder="Confirm password" autocomplete="new-password" />
        <p v-if="userError"   class="settings-error">{{ userError }}</p>
        <p v-if="userSuccess" class="settings-ok">{{ userSuccess }}</p>
        <button type="submit" :disabled="userBusy">{{ userBusy ? 'Creating…' : 'Create User' }}</button>
      </form>

      <!-- Admin -->
      <div v-if="tab === 'admin'" class="admin-panel">

        <p v-if="adminError" class="settings-error">{{ adminError }}</p>

        <!-- Stats -->
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

        <!-- Orphan cleanup -->
        <div class="admin-section">
          <div class="admin-section-head">Orphan Files</div>
          <p class="admin-muted">Files in uploads that no card references.</p>
          <div class="admin-row">
            <button class="admin-btn" :disabled="orphanBusy" @click="scanOrphans">
              {{ orphanBusy && !orphanScan ? 'Scanning…' : 'Scan' }}
            </button>
            <button
              v-if="orphanScan && orphanScan.count > 0"
              class="admin-btn admin-btn-red"
              :disabled="orphanBusy"
              @click="deleteOrphans"
            >{{ orphanBusy ? 'Deleting…' : `Delete ${orphanScan.count} file${orphanScan.count !== 1 ? 's' : ''}` }}</button>
          </div>
          <p v-if="orphanScan && orphanScan.count === 0" class="admin-ok">No orphans found.</p>
          <p v-if="orphanResult" class="admin-ok">{{ orphanResult }}</p>
        </div>

        <!-- Export seed -->
        <div class="admin-section">
          <div class="admin-section-head">Export Seed</div>
          <p class="admin-muted">Write current DB cards to the server's seed file. Run this locally before pushing.</p>
          <button class="admin-btn" :disabled="exportBusy" @click="exportSeed">
            {{ exportBusy ? 'Exporting…' : 'Export to Seed File' }}
          </button>
          <p v-if="exportResult" class="admin-ok">{{ exportResult }}</p>
        </div>

        <!-- Suggestions -->
        <div class="admin-section">
          <div class="admin-section-head">Tune Suggestions <span v-if="suggestions.length" class="admin-badge">{{ suggestions.length }}</span></div>
          <p v-if="suggestionsError" class="settings-error">{{ suggestionsError }}</p>
          <div v-if="suggestionsBusy" class="admin-muted">Loading…</div>
          <div v-else-if="!suggestions.length" class="admin-muted">No suggestions yet.</div>
          <div v-else class="admin-suggestions">
            <div v-for="s in suggestions" :key="s.id" class="admin-suggestion">
              <div class="admin-suggestion-head">
                <span class="admin-suggestion-title">{{ s.title }}</span>
                <span class="admin-suggestion-meta">{{ s.cardId }} · {{ s.submittedAt.slice(0,10) }}</span>
              </div>
              <div v-if="s.credit" class="admin-suggestion-credit">{{ s.credit }}</div>
              <button class="admin-btn admin-btn-red admin-suggestion-dismiss" @click="dismissSuggestion(s.id)">Dismiss</button>
            </div>
          </div>
        </div>

        <!-- Reload seed -->
        <div class="admin-section">
          <div class="admin-section-head">Reload from Seed</div>
          <p class="admin-muted">Apply the deployed seed file to the live DB — upserts all cards, removes deleted ones. Run this on production after a deploy.</p>
          <button class="admin-btn" :disabled="reloadBusy" @click="reloadSeed">
            {{ reloadBusy ? 'Reloading…' : 'Reload from Seed' }}
          </button>
          <p v-if="reloadResult" class="admin-ok">{{ reloadResult }}</p>
        </div>

      </div>

      <!-- Migrate -->
      <div v-if="tab === 'migrate'" class="admin-panel">

        <!-- Category normalization -->
        <div class="admin-section">
          <div class="admin-section-head">Upgrade Category Names</div>
          <p class="admin-muted">Rename legacy category strings to canonical values.</p>
          <div class="mig-card-list">
            <div v-for="s in migrateStatus" :key="s.card.id" class="mig-card-row">
              <span class="mig-card-name">{{ s.card.name }}</span>
              <span v-if="s.needsCategories" class="mig-badge mig-badge--warn">⚠ needs fix</span>
              <span v-else class="mig-badge mig-badge--ok">✓</span>
            </div>
            <div v-if="migrateStatus.every(s => !s.needsCategories)" class="admin-muted">All canonical.</div>
          </div>
          <div v-if="migrateStatus.some(s => s.needsCategories)" class="admin-row">
            <button class="admin-btn" :disabled="catBusy" @click="fixAllCategories">
              {{ catBusy ? 'Fixing…' : 'Fix All' }}
            </button>
          </div>
          <p v-if="catResult" class="admin-ok">{{ catResult }}</p>
        </div>

        <!-- Adjustment row migration -->
        <div class="admin-section">
          <div class="admin-section-head">Adjustment Rows</div>
          <p class="admin-muted">Convert free-text rows to structured slider format.</p>

          <!-- Card selector -->
          <template v-if="!adjCardId">
            <div class="mig-card-list">
              <div
                v-for="s in migrateStatus.filter(s => s.legacyCount > 0)"
                :key="s.card.id"
                class="mig-card-row"
              >
                <span class="mig-card-name">{{ s.card.name }}</span>
                <span class="mig-badge mig-badge--warn">⚠ {{ s.legacyCount }} row{{ s.legacyCount !== 1 ? 's' : '' }}</span>
                <button class="admin-btn mig-btn-sm" @click="openAdjCard(s.card.id)">Migrate →</button>
              </div>
              <div v-if="migrateStatus.every(s => s.legacyCount === 0)" class="admin-muted">All rows structured.</div>
            </div>
            <p v-if="adjResult" class="admin-ok">{{ adjResult }}</p>
          </template>

          <!-- Row-by-row form -->
          <template v-else>
            <div class="mig-form-header">
              <span class="mig-form-title">{{ cards.byId(adjCardId)?.name }}</span>
              <span class="mig-form-progress">Row {{ adjRowIdx + 1 }} / {{ adjLegacyRows.length }}</span>
              <button class="admin-btn mig-btn-sm" @click="adjCardId = null">← Back</button>
            </div>

            <!-- Source row context -->
            <div v-if="adjCurrentRow" class="mig-source-row">
              <div class="mig-source-label">{{ adjCurrentRow.name }}</div>
              <div class="mig-source-desc">{{ adjCurrentRow.description }}</div>
            </div>

            <!-- Already handled? -->
            <div v-if="adjResults.has(adjRowIdx)" class="mig-handled">
              <span v-if="adjResults.get(adjRowIdx) === 'skip'" class="mig-badge mig-badge--skip">Skipped</span>
              <template v-else>
                <span v-for="(r, i) in adjCurrentSaved" :key="i" class="mig-badge mig-badge--ok">
                  {{ r.group }} {{ r.label }}
                </span>
              </template>
            </div>

            <!-- Structured entry form -->
            <div v-if="!adjAllHandled" class="mig-form">
              <div class="mig-field-row">
                <label class="mig-label">Tab</label>
                <select v-model="adjDraft.tab" class="mig-select" @change="onAdjTabChange">
                  <option v-for="t in TABS" :key="t" :value="t">{{ t }}</option>
                </select>
              </div>
              <div class="mig-field-row">
                <label class="mig-label">Group</label>
                <input v-model="adjDraft.group" class="mig-input" placeholder="e.g. Front Anti-Roll Bar" />
              </div>
              <div class="mig-field-row">
                <label class="mig-label">Label</label>
                <input v-model="adjDraft.label" class="mig-input" placeholder="e.g. Front" />
              </div>
              <div class="mig-field-row">
                <label class="mig-label">Unit</label>
                <input v-model="adjDraft.unit" class="mig-input mig-input--short" placeholder="° or psi or blank" />
              </div>
              <div class="mig-nums-row">
                <div class="mig-num">
                  <label class="mig-label">Min</label>
                  <input v-model.number="adjDraft.min" type="number" class="mig-input mig-input--num" />
                </div>
                <div class="mig-num">
                  <label class="mig-label">Max</label>
                  <input v-model.number="adjDraft.max" type="number" class="mig-input mig-input--num" />
                </div>
                <div class="mig-num">
                  <label class="mig-label">Stock</label>
                  <input v-model.number="adjDraft.stock" type="number" class="mig-input mig-input--num" />
                </div>
                <div class="mig-num">
                  <label class="mig-label">Value</label>
                  <input v-model.number="adjDraft.value" type="number" class="mig-input mig-input--num" />
                </div>
                <div class="mig-num">
                  <label class="mig-label">Step</label>
                  <input v-model.number="adjDraft.step" type="number" class="mig-input mig-input--num" />
                </div>
              </div>
              <div class="admin-row">
                <button class="admin-btn" @click="saveRow">Save Row</button>
                <button
                  class="admin-btn"
                  :disabled="adjCurrentSaved.length === 0"
                  @click="nextRow"
                >Next →</button>
                <button class="admin-btn" @click="skipRow">Skip</button>
              </div>
            </div>

            <!-- Commit when all handled -->
            <div v-if="adjAllHandled" class="mig-commit">
              <p class="admin-muted">All rows handled. Commit to save the card.</p>
              <button class="admin-btn" :disabled="adjBusy" @click="commitAdjMigration">
                {{ adjBusy ? 'Saving…' : 'Commit Migration' }}
              </button>
              <p v-if="adjResult" class="admin-ok">{{ adjResult }}</p>
            </div>
          </template>
        </div>

      </div>

      <div class="settings-footer">
        <button class="logout-btn" @click="logout">Sign Out</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel { max-width: 360px; max-height: 88vh; overflow-y: auto; overscroll-behavior: contain; }
.settings-panel--wide { max-width: 540px; }

/* Tabs */
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
.settings-tabs button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

/* Form — mirrors LoginModal scoped styles */
.settings-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 2px 2px;
}
.settings-form input {
  padding: 9px 11px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--fg);
  font-family: inherit;
  font-size: 14px;
}
.settings-form input:focus {
  outline: none;
  border-color: var(--accent);
}
.settings-form button[type='submit'] {
  margin-top: 4px;
  padding: 9px 12px;
  border-radius: 4px;
  border: 1px solid var(--build-it-border);
  background: var(--build-it-bg);
  color: #fff;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
}
.settings-form button[type='submit']:hover:not(:disabled) { background: var(--build-it-bg-hover); }
.settings-form button[type='submit']:disabled { opacity: 0.6; cursor: default; }

.settings-error { color: var(--danger-bright); font-size: 13px; margin: 0; }
.settings-ok    { color: var(--accent);          font-size: 13px; margin: 0; }

.admin-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 4px 2px 2px;
}
.admin-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.admin-section-head {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent);
  padding-bottom: 4px;
  border-bottom: 1px solid var(--panel-edge);
}
.admin-stats-table {
  width: 100%;
  border-collapse: collapse;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
}
.admin-stats-table td { padding: 3px 0; }
.admin-stats-table td:first-child { color: var(--muted); }
.admin-stats-table td:last-child { text-align: right; color: var(--fg); }
.admin-muted {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--muted);
  margin: 0;
}
.admin-ok {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--accent);
  margin: 0;
}
.admin-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
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
.admin-btn-red {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 35%, transparent);
  color: var(--danger-bright);
}
.admin-btn-red:hover:not(:disabled) {
  background: color-mix(in srgb, var(--danger) 60%, transparent);
  border-color: var(--danger-bright);
  box-shadow: 0 0 12px color-mix(in srgb, var(--danger-bright) 50%, transparent);
}

.settings-footer {
  margin-top: 20px;
  padding-top: 14px;
  border-top: 1px solid var(--panel-edge);
}
.logout-btn {
  width: 100%;
  padding: 8px 12px;
  border-radius: 4px;
  border: 2px solid #7a0000;
  background: #5c0000;
  color: #fff;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, box-shadow 0.15s;
}
.logout-btn:hover {
  background: #cc0000;
  border-color: #ff4444;
  box-shadow: 0 0 16px rgba(200, 0, 0, 0.85);
}

.admin-badge {
  display: inline-block;
  background: var(--highlight);
  color: #fff;
  font-size: 9px;
  border-radius: 100px;
  padding: 1px 6px;
  margin-left: 6px;
  vertical-align: middle;
}
.admin-suggestions { display: flex; flex-direction: column; gap: 8px; }
.admin-suggestion {
  border: 1px solid var(--panel-edge);
  border-radius: 5px;
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.admin-suggestion-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 8px;
}
.admin-suggestion-title {
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: 600;
  color: var(--fg);
}
.admin-suggestion-meta {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted);
  white-space: nowrap;
}
.admin-suggestion-credit {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--accent);
}
.admin-suggestion-dismiss { align-self: flex-start; margin-top: 4px; }

/* Migration tab */
.mig-card-list { display: flex; flex-direction: column; gap: 4px; }
.mig-card-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
}
.mig-card-name { flex: 1; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mig-badge {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 10px;
  flex-shrink: 0;
}
.mig-badge--ok   { background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent); }
.mig-badge--warn { background: color-mix(in srgb, var(--highlight) 15%, transparent); color: var(--highlight); }
.mig-badge--skip { background: color-mix(in srgb, var(--muted) 15%, transparent); color: var(--muted); }
.mig-btn-sm { padding: 3px 10px; font-size: 10px; flex-shrink: 0; }

.mig-form-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.mig-form-title {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mig-form-progress {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted);
  flex-shrink: 0;
}

.mig-source-row {
  border-left: 2px solid var(--highlight);
  padding: 6px 10px;
  margin-bottom: 10px;
  background: color-mix(in srgb, var(--highlight) 6%, transparent);
  border-radius: 0 4px 4px 0;
}
.mig-source-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  font-weight: 600;
  color: var(--fg);
}
.mig-source-desc {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted-light);
  margin-top: 2px;
}

.mig-handled { margin-bottom: 8px; }

.mig-form { display: flex; flex-direction: column; gap: 8px; }
.mig-field-row { display: flex; align-items: center; gap: 8px; }
.mig-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  width: 46px;
  flex-shrink: 0;
}
.mig-input {
  flex: 1;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 5px 8px;
}
.mig-input:focus { outline: none; border-color: var(--accent); }
.mig-input--short { max-width: 90px; }
.mig-input--num { width: 72px; flex: none; }
.mig-select {
  flex: 1;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 5px 8px;
}
.mig-select:focus { outline: none; border-color: var(--accent); }

.mig-nums-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.mig-num { display: flex; flex-direction: column; gap: 3px; }
.mig-num .mig-label { width: auto; }

.mig-commit { display: flex; flex-direction: column; gap: 8px; }
</style>
