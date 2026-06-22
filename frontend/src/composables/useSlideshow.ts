import { ref, computed, onMounted, onBeforeUnmount, type Ref } from 'vue'
import type { CardImage } from '../types'

// Slideshow timing — mirrors the constants in the original single-file app.
const SOLID = 2
const DISSOLVE = 2
const SLIDE = SOLID + DISSOLVE // seconds each slide is shown
const BUTTON_REVEAL = 2 // "Autoplaying" button lingers this long before fading
const BUTTON_FADE = 2 // how long the button's fade-out takes
const BRIGHT = 1
const DIM = 0.25
const DIM_FADE = 1

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
  const ordered = computed(() => [...images.value].sort((a, b) => a.order - b.order))
  const index = ref(0)
  const playing = ref(false)
  const toggleIcon = ref('▶')
  const toggleLabel = ref('Paused')

  let timer: number | undefined
  let revealTimer: number | undefined
  let fadeTimer: number | undefined
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
    timer = revealTimer = fadeTimer = undefined
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

  // The play/pause button: an explicit user action that sticks across scrolling.
  function toggle() {
    clearTimers()
    setSettled(false)
    if (playing.value) {
      userPaused = true
      pause()
    } else {
      userPaused = false
      play()
    }
    updateToggle()
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
    if (stageRef.value) obs.observe(stageRef.value)
    onBeforeUnmount(() => {
      obs.disconnect()
      clearTimers()
    })
  })

  return { ordered, index, playing, toggleIcon, toggleLabel, toggle, onThumb }
}
