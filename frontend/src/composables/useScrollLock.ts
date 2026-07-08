export function useScrollLock() {
  function lockScroll()   { document.body.style.overflow = 'hidden' }
  function unlockScroll() { document.body.style.overflow = '' }
  return { lockScroll, unlockScroll }
}
