import { onBeforeUnmount, watch, type Ref } from 'vue'

// Keeps the screen awake while `active` is true on a touch device (decision D4):
// entering a tune from the game takes minutes of reading, not touching, and the
// phone would dim. The browser drops the lock whenever the page is hidden, so it
// is re-requested when the page comes back. No API (older Safari) → no-op.
export function useWakeLock(active: Ref<boolean>) {
  const touch = window.matchMedia('(hover: none), (pointer: coarse)')
  let sentinel: WakeLockSentinel | null = null

  async function acquire() {
    if (sentinel || !('wakeLock' in navigator) || !touch.matches || document.visibilityState !== 'visible') return
    try {
      sentinel = await navigator.wakeLock.request('screen')
      sentinel.addEventListener('release', () => { sentinel = null })
    } catch {
      sentinel = null // denied (e.g. low battery) — the screen just dims as usual
    }
  }
  function release() {
    sentinel?.release().catch(() => {})
    sentinel = null
  }
  function onVisibility() {
    if (active.value && document.visibilityState === 'visible') acquire()
  }

  watch(active, (on) => (on ? acquire() : release()), { immediate: true })
  document.addEventListener('visibilitychange', onVisibility)
  onBeforeUnmount(() => {
    document.removeEventListener('visibilitychange', onVisibility)
    release()
  })
}
