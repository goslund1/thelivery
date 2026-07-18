import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '../api'

// Holds the JWT + current user. Token persists in localStorage so a refresh
// keeps you signed in. (We rely on the upload XSS fix + nosniff to keep
// localStorage safe from script injection.)
export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('auth_token'))
  const username = ref<string | null>(localStorage.getItem('auth_user'))
  const role = ref<string | null>(localStorage.getItem('auth_role'))
  // Temp-password flow: while set, ForcePasswordModal blocks the app (and the
  // backend rejects everything except the change-password endpoint anyway).
  const mustChangePassword = ref(localStorage.getItem('auth_must_change') === '1')
  const isAuthenticated = computed(() => !!token.value)
  // UI convenience only — the backend enforces roles on every request.
  const isAdmin = computed(() => role.value === 'admin')

  async function login(user: string, password: string) {
    const res = await api.login(user, password)
    token.value = res.token
    username.value = res.username
    role.value = res.role
    mustChangePassword.value = res.mustChangePassword
    localStorage.setItem('auth_token', res.token)
    localStorage.setItem('auth_user', res.username)
    localStorage.setItem('auth_role', res.role)
    localStorage.setItem('auth_must_change', res.mustChangePassword ? '1' : '0')
  }

  function passwordChanged() {
    mustChangePassword.value = false
    localStorage.setItem('auth_must_change', '0')
  }

  function logout() {
    token.value = null
    username.value = null
    role.value = null
    mustChangePassword.value = false
    localStorage.removeItem('auth_token')
    localStorage.removeItem('auth_user')
    localStorage.removeItem('auth_role')
    localStorage.removeItem('auth_must_change')
  }

  return { token, username, role, mustChangePassword, isAuthenticated, isAdmin, login, passwordChanged, logout }
})
