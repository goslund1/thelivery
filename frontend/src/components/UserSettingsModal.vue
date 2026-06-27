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
      <form v-if="tab === 'password'" class="login-form" @submit.prevent="submitChangePassword">
        <input v-model="currentPw" type="password" placeholder="Current password" autocomplete="current-password" />
        <input v-model="newPw"     type="password" placeholder="New password" autocomplete="new-password" />
        <input v-model="confirmPw" type="password" placeholder="Confirm new password" autocomplete="new-password" />
        <p v-if="pwError"   class="login-error">{{ pwError }}</p>
        <p v-if="pwSuccess" class="settings-ok">{{ pwSuccess }}</p>
        <button type="submit" :disabled="pwBusy">{{ pwBusy ? 'Saving…' : 'Update Password' }}</button>
      </form>

      <!-- Create User -->
      <form v-if="tab === 'create'" class="login-form" @submit.prevent="submitCreateUser">
        <input v-model="newUsername" type="text"     placeholder="Username" autocomplete="off" />
        <input v-model="newUserPw"   type="password" placeholder="Password" autocomplete="new-password" />
        <input v-model="newUserConfirm" type="password" placeholder="Confirm password" autocomplete="new-password" />
        <p v-if="userError"   class="login-error">{{ userError }}</p>
        <p v-if="userSuccess" class="settings-ok">{{ userSuccess }}</p>
        <button type="submit" :disabled="userBusy">{{ userBusy ? 'Creating…' : 'Create User' }}</button>
      </form>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  max-width: 360px;
}
.settings-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
}
.settings-tabs button {
  flex: 1;
  padding: 6px 8px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: none;
  color: var(--text);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  cursor: pointer;
  opacity: 0.5;
}
.settings-tabs button.active {
  border-color: var(--gold);
  color: var(--gold);
  opacity: 1;
}
.settings-ok {
  color: var(--gold);
  font-size: 13px;
  margin: 0;
}
</style>
