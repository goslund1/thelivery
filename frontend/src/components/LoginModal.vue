<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useModalStore } from '../stores/modal'
import { useAuthStore } from '../stores/auth'

const modal = useModalStore()
const auth = useAuthStore()

const username = ref('')
const password = ref('')
const error = ref('')
const busy = ref(false)
const userRef = ref<HTMLInputElement | null>(null)

async function submit() {
  if (busy.value) return
  error.value = ''
  busy.value = true
  try {
    await auth.login(username.value.trim(), password.value)
    password.value = ''
    modal.onLoginSuccess()
  } catch {
    error.value = 'Invalid username or password.'
  } finally {
    busy.value = false
  }
}

watch(
  () => modal.loginOpen,
  async (open) => {
    if (open) {
      error.value = ''
      username.value = auth.username ?? ''
      password.value = ''
      await nextTick()
      userRef.value?.focus()
    }
  },
)
</script>

<template>
  <div v-if="modal.loginOpen" class="image-picker open" @click.self="modal.closeLogin()">
    <div class="image-picker-panel login-panel">
      <div class="image-picker-head">
        <span>Sign in to edit</span>
        <button class="image-picker-close" aria-label="Close" @click="modal.closeLogin()">×</button>
      </div>
      <form class="login-form" @submit.prevent="submit">
        <input ref="userRef" v-model="username" type="text" placeholder="Username" autocomplete="username" />
        <input v-model="password" type="password" placeholder="Password" autocomplete="current-password" />
        <p v-if="error" class="login-error">{{ error }}</p>
        <button type="submit" :disabled="busy">{{ busy ? 'Signing in…' : 'Sign in' }}</button>
      </form>
    </div>
  </div>
</template>

<style scoped>
.login-panel {
  max-width: 340px;
}
.login-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 2px 2px;
}
.login-form input {
  padding: 9px 11px;
  border-radius: 4px;
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--fg);
  font-size: 14px;
}
.login-form input:focus {
  outline: none;
  border-color: var(--accent);
}
.login-form button[type='submit'] {
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
.login-form button[type='submit']:disabled {
  opacity: 0.6;
  cursor: default;
}
.login-error {
  color: var(--danger-bright);
  font-size: 13px;
  margin: 0;
}
</style>
