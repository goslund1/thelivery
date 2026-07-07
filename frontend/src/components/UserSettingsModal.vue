<script setup lang="ts">
import { ref } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useAuthStore } from '../stores/auth'
import { api } from '../api'

const ui   = useUiStore()
const modal = useModalStore()
const auth  = useAuthStore()

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
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    pwError.value = msg.includes('incorrect') ? 'Current password is incorrect.' : 'Failed to update password.'
  } finally {
    pwBusy.value = false
  }
}

function close() {
  modal.closeSettings()
  pwError.value = ''; pwSuccess.value = ''
  currentPw.value = ''; newPw.value = ''; confirmPw.value = ''
}

function logout() {
  if (ui.isEditing) ui.toggleEdit()
  auth.logout()
  modal.closeSettings()
}

function openAdmin() {
  modal.closeSettings()
  modal.openAdminPanel()
}
</script>

<template>
  <div v-if="modal.settingsOpen" class="image-picker open" @click.self="close()">
    <div class="image-picker-panel settings-panel">
      <div class="image-picker-head">
        <span>Account — {{ auth.username }}</span>
        <button class="image-picker-close" aria-label="Close" @click="close()">×</button>
      </div>

      <form class="settings-form" @submit.prevent="submitChangePassword">
        <input v-model="currentPw"  type="password" placeholder="Current password"     autocomplete="current-password" />
        <input v-model="newPw"      type="password" placeholder="New password"         autocomplete="new-password" />
        <input v-model="confirmPw"  type="password" placeholder="Confirm new password" autocomplete="new-password" />
        <p v-if="pwError"   class="settings-error">{{ pwError }}</p>
        <p v-if="pwSuccess" class="settings-ok">{{ pwSuccess }}</p>
        <button type="submit" :disabled="pwBusy">{{ pwBusy ? 'Saving…' : 'Update Password' }}</button>
      </form>

      <div class="settings-footer">
        <button v-if="auth.isAuthenticated" class="admin-link-btn" @click="openAdmin">Admin Panel →</button>
        <button class="logout-btn" @click="logout">Sign Out</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  max-width: 320px;
  max-height: 88vh;
  overflow-y: auto;
  overscroll-behavior: contain;
}

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
.settings-form input:focus { outline: none; border-color: var(--accent); }
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
.settings-ok    { color: var(--accent);         font-size: 13px; margin: 0; }

.settings-footer {
  margin-top: 20px;
  padding-top: 14px;
  border-top: 1px solid var(--panel-edge);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.admin-link-btn {
  width: 100%;
  padding: 8px 12px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--accent);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  text-align: left;
}
.admin-link-btn:hover { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 8%, transparent); }

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
