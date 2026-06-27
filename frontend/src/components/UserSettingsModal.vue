<script setup lang="ts">
import { ref } from 'vue'
import { useUiStore } from '../stores/ui'
import { useAuthStore } from '../stores/auth'
import { api } from '../api'

const ui = useUiStore()
const auth = useAuthStore()

type Tab = 'password' | 'create'
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
</script>

<template>
  <div v-if="ui.settingsOpen" class="image-picker open" @click.self="close()">
    <div class="image-picker-panel settings-panel">
      <div class="image-picker-head">
        <span>Account — {{ auth.username }}</span>
        <button class="image-picker-close" aria-label="Close" @click="close()">×</button>
      </div>

      <div class="settings-tabs">
        <button :class="{ active: tab === 'password' }" @click="tab = 'password'">Change Password</button>
        <button :class="{ active: tab === 'create' }" @click="tab = 'create'">Create User</button>
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
