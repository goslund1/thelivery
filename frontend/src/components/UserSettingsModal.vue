<script setup lang="ts">
import { ref } from 'vue'
import { useUiStore } from '../stores/ui'
import { useAuthStore } from '../stores/auth'
import { api } from '../api'

const ui = useUiStore()
const auth = useAuthStore()

type Tab = 'password' | 'create' | 'admin'
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
  ui.closeSettings()
  pwError.value = ''; pwSuccess.value = ''
  userError.value = ''; userSuccess.value = ''
  currentPw.value = ''; newPw.value = ''; confirmPw.value = ''
  newUsername.value = ''; newUserPw.value = ''; newUserConfirm.value = ''
}

function logout() {
  if (ui.isEditing) ui.toggleEdit()
  auth.logout()
  ui.closeSettings()
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
}
</script>

<template>
  <div v-if="ui.settingsOpen" class="image-picker open" @click.self="close()">
    <div class="image-picker-panel settings-panel">
      <div class="image-picker-head">
        <span>Account — {{ auth.username }}</span>
        <button class="image-picker-close" aria-label="Close" @click="close()">×</button>
      </div>

      <div class="settings-tabs">
        <button :class="{ active: tab === 'password' }" @click="tab = 'password'">Password</button>
        <button :class="{ active: tab === 'create' }" @click="tab = 'create'">Users</button>
        <button :class="{ active: tab === 'admin' }" @click="onTabAdmin">Admin</button>
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

      <div class="settings-footer">
        <button class="logout-btn" @click="logout">Sign Out</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel { max-width: 360px; }

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
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.settings-tabs button:hover { color: var(--paper); }
.settings-tabs button.active {
  color: var(--gold);
  border-bottom-color: var(--gold);
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
  color: var(--paper);
  font-family: inherit;
  font-size: 14px;
}
.settings-form input:focus {
  outline: none;
  border-color: var(--gold);
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
.settings-ok    { color: var(--gold);          font-size: 13px; margin: 0; }

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
  color: var(--gold);
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
.admin-stats-table td:first-child { color: var(--steel); }
.admin-stats-table td:last-child { text-align: right; color: var(--paper); }
.admin-muted {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--steel);
  margin: 0;
}
.admin-ok {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--gold);
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
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.admin-btn:hover:not(:disabled) { border-color: var(--gold); color: var(--gold); }
.admin-btn:disabled { opacity: 0.5; cursor: default; }
.admin-btn-red {
  border-color: #7a0000;
  background: #5c0000;
  color: #fff;
}
.admin-btn-red:hover:not(:disabled) {
  background: #cc0000;
  border-color: #ff4444;
  box-shadow: 0 0 12px rgba(200, 0, 0, 0.7);
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
</style>
