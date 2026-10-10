import { ref, computed, onMounted, onBeforeUnmount, type Ref } from 'vue'
import type { CardImage } from '../types'
import { isTouch } from './isTouch'

// Slideshow timing — mirrors the constants in the original single-file app.
const SOLID = 2
const DISSOLVE = 2
const SLIDE = SOLID + DISSOLVE // seconds each slide is shown
const BUTTON_REVEAL = 2 // "Autoplaying" button lingers this long before fading
const BUTTON_FADE = 2 // how long the button's fade-out takes
const BRIGHT = 1
const DIM = 0.25
const DIM_FADE = 1
// The pill sleeps while playing (Part C): a mouse over the stage or a touch wakes it.
const RESUME_SHOW = 1.5 // after a manual resume, "Playing" shows this long, then sleeps
const MOUSE_IDLE = 3 // a mouse resting on a playing stage this long puts the pill back to sleep
const TOUCH_WAKE = 1.5 // a touch or a scroll shows the pill this long, then the normal fade
const SCROLL_WAKE_THROTTLE = 100 // ms between scroll wakes

// Drives one card's gallery: current slide, autoplay (only while the card is on
// screen), the progress bar, and the in-frame play button's reveal→dissolve
// choreography. Ports the original getImages/showSlide/play/pause/startProgress
// plus the IntersectionObserver reveal sequence.
export function useSlideshow(
  images: Ref<CardImage[]>,
  stageRef: Ref<HTMLElement | null>,
  barRef: Ref<HTMLElement | null>,
  toggleRef: Ref<HTMLElement | null>,
) {
  const ordered = computed(() =>
    [...images.value]
      .filter((img) => img.included !== false)
      .sort((a, b) => a.order - b.order),
  )
  const index = ref(0)
  const playing = ref(false)
  const toggleIcon = ref('▶')
  const toggleLabel = ref('Paused')

  let timer: number | undefined
  let revealTimer: number | undefined
  let fadeTimer: number | undefined
  let sleepTimer: number | undefined
  let idleTimer: number | undefined
  let wakeTimer: number | undefined
  let userPaused = false // explicit pause/thumb-click; don't auto-resume on scroll
  let visible = false

  // The lead/feature image is order 0 — i.e. the first in the ordered list.
  function leadIndex() {
    return 0
  }
  function show(i: number) {
    const n = ordered.value.length
    if (!n) return
    index.value = ((i % n) + n) % n
  }
  function next() {
    show(index.value + 1)
    startProgress()
  }

  // Progress bar: flash bright + full width with no transition (an anchor point).
  function groundProgress() {
    const bar = barRef.value
    if (!bar) return
    bar.style.transition = 'none'
    bar.style.opacity = String(BRIGHT)
    bar.style.width = '100%'
  }
  // Animate width 100% -> 0 over one slide, fading bright -> dim.
  function startProgress() {
    const bar = barRef.value
    if (!bar) return
    bar.style.transition = 'none'
    bar.style.width = '100%'
    bar.style.opacity = String(BRIGHT)
    void bar.offsetWidth // force reflow so the next transition takes effect
    bar.style.transition = `width ${SLIDE}s linear, opacity ${DIM_FADE}s ease`
    bar.style.width = '0%'
    bar.style.opacity = String(DIM)
  }
  function stopProgress() {
    const bar = barRef.value
    if (!bar) return
    bar.style.transition = 'opacity .3s ease'
    bar.style.opacity = '0'
  }

  function setSettled(on: boolean) {
    stageRef.value?.classList.toggle('settled', on)
  }
  function updateToggle() {
    toggleIcon.value = playing.value ? '❙❙' : '▶'
    toggleLabel.value = playing.value ? 'Playing' : 'Paused'
  }
  function clearTimers() {
    if (timer) clearInterval(timer)
    if (revealTimer) clearTimeout(revealTimer)
    if (fadeTimer) clearTimeout(fadeTimer)
    if (sleepTimer) clearTimeout(sleepTimer)
    timer = revealTimer = fadeTimer = sleepTimer = undefined
  }

  function play() {
    if (playing.value || ordered.value.length < 2) return
    playing.value = true
    startProgress()
    timer = window.setInterval(next, SLIDE * 1000)
  }
  function pause() {
    playing.value = false
    if (timer) {
      clearInterval(timer)
      timer = undefined
    }
    stopProgress()
  }

  // Reveal choreography: show the button ("Autoplaying") over a grounded bar;
  // after BUTTON_REVEAL the button slow-fades out as autoplay starts; after
  // BUTTON_FADE the toggle text settles into its normal playing state.
  function beginReveal() {
    clearTimers()
    playing.value = false
    setSettled(false)
    toggleIcon.value = ''
    toggleLabel.value = 'Autoplaying'
    groundProgress()
    if (ordered.value.length < 2) return // single image: nothing to autoplay
    revealTimer = window.setTimeout(() => {
      revealTimer = undefined
      const btn = toggleRef.value
      if (btn) btn.style.transition = `opacity ${BUTTON_FADE}s ease` // slow, just this once
      setSettled(true)
      play()
      fadeTimer = window.setTimeout(() => {
        fadeTimer = undefined
        updateToggle()
        if (btn) btn.style.transition = '' // restore the fast default for hover
      }, BUTTON_FADE * 1000)
    }, BUTTON_REVEAL * 1000)
  }

  function resumeAutoplay() {
    if (userPaused || playing.value) return
    beginReveal()
  }
  function suspendAutoplay() {
    clearTimers()
    pause()
    setSettled(false)
    show(leadIndex())
    updateToggle()
  }

  // The play/pause button (and a click on the stage): an explicit user action that
  // sticks across scrolling. Paused, the pill stays up (C4). Resumed, it shows
  // "Playing" briefly and then sleeps like after the first reveal (C1).
  function toggle() {
    clearTimers()
    setSettled(false)
    if (playing.value) {
      userPaused = true
      pause()
    } else {
      userPaused = false
      play()
      if (playing.value) sleepTimer = window.setTimeout(() => {
        sleepTimer = undefined
        setSettled(true)
      }, RESUME_SHOW * 1000)
    }
    updateToggle()
  }

  // C2 mouse: hovering a playing stage wakes the pill (CSS, .stage.settled:hover);
  // resting the mouse there for MOUSE_IDLE puts it back to sleep (.idle) until
  // it moves again.
  function setIdle(on: boolean) {
    stageRef.value?.classList.toggle('idle', on)
  }
  function onPointerMove(e: PointerEvent) {
    if (e.pointerType !== 'mouse') return
    setIdle(false)
    if (idleTimer) clearTimeout(idleTimer)
    idleTimer = window.setTimeout(() => setIdle(true), MOUSE_IDLE * 1000)
  }
  function onPointerLeave() {
    if (idleTimer) clearTimeout(idleTimer)
    idleTimer = undefined
    setIdle(false)
  }

  // C3 touch: no hover hold. A touch on the stage, or a page scroll while this
  // stage is on screen, wakes the pill for TOUCH_WAKE; a new wake restarts it.
  function wake() {
    if (!playing.value) return // paused: the pill is up anyway
    stageRef.value?.classList.add('woken')
    if (wakeTimer) clearTimeout(wakeTimer)
    wakeTimer = window.setTimeout(() => {
      wakeTimer = undefined
      stageRef.value?.classList.remove('woken')
    }, TOUCH_WAKE * 1000)
  }
  let lastScrollWake = 0
  function onScroll() {
    if (!visible) return
    const now = performance.now()
    if (now - lastScrollWake < SCROLL_WAKE_THROTTLE) return
    lastScrollWake = now
    wake()
  }
  // Clicking a thumbnail is an explicit pause + jump (won't auto-resume).
  function onThumb(i: number) {
    clearTimers()
    userPaused = true
    pause()
    setSettled(false)
    show(i)
    updateToggle()
  }

  onMounted(() => {
    index.value = leadIndex()
    // Autoplay only while the card is visible: reveal on enter, suspend on exit.
    const obs = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          visible = e.isIntersecting
          if (visible) resumeAutoplay()
          else suspendAutoplay()
        })
      },
      { threshold: 0.5 },
    )
    const stage = stageRef.value
    if (stage) obs.observe(stage)
    if (stage) {
      stage.addEventListener('pointermove', onPointerMove)
      stage.addEventListener('pointerleave', onPointerLeave)
    }
    if (isTouch) {
      stage?.addEventListener('touchstart', wake, { passive: true })
      window.addEventListener('scroll', onScroll, { passive: true })
    }
    onBeforeUnmount(() => {
      obs.disconnect()
      clearTimers()
      if (idleTimer) clearTimeout(idleTimer)
      if (wakeTimer) clearTimeout(wakeTimer)
      stage?.removeEventListener('pointermove', onPointerMove)
      stage?.removeEventListener('pointerleave', onPointerLeave)
      stage?.removeEventListener('touchstart', wake)
      window.removeEventListener('scroll', onScroll)
    })
  })

  return { ordered, index, playing, toggleIcon, toggleLabel, toggle, onThumb }
}
