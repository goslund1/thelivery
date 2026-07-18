<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useAuthStore } from '../stores/auth'
import { api } from '../api'
import { errMsg } from '../utils/errMsg'

// Forced password change after logging in with a temporary password.
// Deliberately NOT registered with the modal store: no Escape, no
// click-outside, no closeTopModal — signing out is the only other way out.
// The backend rejects every request except the change itself while the
// must_change_password flag is set, so this modal is UX, not the enforcement.
const auth = useAuthStore()

const currentPw = ref('')
const newPw = ref('')
const confirmPw = ref('')
const error = ref('')
const busy = ref(false)
const currentRef = ref<HTMLInputElement | null>(null)

async function submit() {
  if (busy.value) return
  error.value = ''
  if (newPw.value !== confirmPw.value) { error.value = 'New passwords do not match.'; return }
  if (newPw.value.length < 8) { error.value = 'New password must be at least 8 characters.'; return }
  if (newPw.value === currentPw.value) { error.value = 'New password must be different from the temporary one.'; return }
  busy.value = true
  try {
    await api.changePassword(currentPw.value, newPw.value)
    auth.passwordChanged()
    currentPw.value = ''; newPw.value = ''; confirmPw.value = ''
  } catch (e) {
    error.value = errMsg(e).includes('incorrect')
      ? 'Temporary password is incorrect.'
      : 'Failed to update password.'
  } finally { busy.value = false }
}

function signOut() {
  auth.logout()
}

watch(
  () => auth.mustChangePassword,
  async (on) => {
    if (on) {
      await nextTick()
      currentRef.value?.focus()
    }
  },
  { immediate: true },
)
</script>

<template>
  <div v-if="auth.mustChangePassword" class="image-picker open float_forcepw_backdrop">
    <div class="image-picker-panel forcepw-panel float_forcepw_panel">
      <div class="image-picker-head">
        <span>Welcome, {{ auth.username }}</span>
      </div>
      <p class="forcepw-intro">
        You signed in with a temporary password. Choose a new one to continue.
      </p>
      <form class="forcepw-form" @submit.prevent="submit">
        <input ref="currentRef" v-model="currentPw" type="password" placeholder="Temporary password" autocomplete="current-password" />
        <input v-model="newPw" type="password" placeholder="New password (8+ characters)" autocomplete="new-password" />
        <input v-model="confirmPw" type="password" placeholder="Confirm new password" autocomplete="new-password" />
        <p v-if="error" class="forcepw-error">{{ error }}</p>
        <button type="submit" :disabled="busy">{{ busy ? 'Saving…' : 'Set Password' }}</button>
      </form>
      <button class="forcepw-signout" @click="signOut">Sign out instead</button>
    </div>
  </div>
</template>

<style scoped>
.forcepw-panel {
  max-width: 340px;
}
.forcepw-intro {
  font-size: 13px;
  color: var(--muted);
  margin: 0 0 10px;
  line-height: 1.5;
}
.forcepw-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 2px 2px;
}
.forcepw-form input {
  padding: 9px 11px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--fg);
  font-family: inherit;
  font-size: 14px;
}
.forcepw-form input:focus { outline: none; border-color: var(--accent); }
.forcepw-form button[type='submit'] {
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
.forcepw-form button[type='submit']:hover:not(:disabled) { background: var(--build-it-bg-hover); }
.forcepw-form button[type='submit']:disabled { opacity: 0.6; cursor: default; }
.forcepw-error {
  color: var(--danger-bright);
  font-size: 13px;
  margin: 0;
}
.forcepw-signout {
  margin-top: 12px;
  align-self: flex-start;
  background: none;
  border: none;
  padding: 0;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  text-decoration: underline;
}
.forcepw-signout:hover { color: var(--fg); }
</style>
