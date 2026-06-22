import { ref, computed, onMounted, onBeforeUnmount, type Ref } from 'vue'
import type { LiveryImage } from '../types'

// Slideshow timing — mirrors the constants in the original single-file app.
const SOLID = 2
const DISSOLVE = 2
const SLIDE = SOLID + DISSOLVE // seconds each slide is shown
const REVEAL = 2 // delay after the card scrolls into view before autoplay starts
const BRIGHT = 1
const DIM = 0.25
const DIM_FADE = 1

// Drives one card's gallery: current slide, autoplay timer, progress bar, and
// the "start autoplay when scrolled into view" behavior. Replaces the original
// getImages/showSlide/nextSlide/play/pause/startProgress functions + the
// IntersectionObserver reveal.
export function useSlideshow(
  images: Ref<LiveryImage[]>,
  stageRef: Ref<HTMLElement | null>,
  barRef: Ref<HTMLElement | null>,
) {
  const ordered = computed(() => [...images.value].sort((a, b) => a.order - b.order))
  const index = ref(0)
  const playing = ref(false)
  let timer: number | undefined
  let revealTimer: number | undefined
  let userPaused = false // user explicitly paused; don't auto-resume on scroll
  let visible = false
  let revealed = false // the first-time reveal delay has already elapsed

  function show(i: number) {
    const n = ordered.value.length
    if (!n) return
    index.value = ((i % n) + n) % n
  }
  function next() {
    show(index.value + 1)
    startProgress()
  }

  // Progress bar: flash bright + full width with no transition.
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

  function play() {
    if (playing.value || ordered.value.length < 2) return
    playing.value = true
    startProgress()
    timer = window.setInterval(next, SLIDE * 1000)
  }
  function pause() {
    playing.value = false
    if (timer) clearInterval(timer)
    timer = undefined
    stopProgress()
  }

  // Begin/resume autoplay because the card is on screen (and the user hasn't
  // manually paused). The very first reveal waits REVEAL seconds; later
  // re-entries into view resume immediately.
  function resumeAutoplay() {
    if (playing.value || userPaused || ordered.value.length < 2) return
    if (!revealed) {
      revealed = true
      groundProgress()
      revealTimer = window.setTimeout(() => {
        revealTimer = undefined
        if (visible && !userPaused) play()
      }, REVEAL * 1000)
    } else {
      play()
    }
  }
  // Stop advancing because the card scrolled off screen. Not a user pause, so
  // it will resume automatically when the card comes back into view.
  function suspendAutoplay() {
    if (revealTimer) {
      clearTimeout(revealTimer)
      revealTimer = undefined
    }
    pause()
  }

  // The play/pause button: an explicit user action that sticks across scrolling.
  function toggle() {
    if (playing.value) {
      userPaused = true
      pause()
    } else {
      userPaused = false
      revealed = true
      if (visible) play()
    }
  }
  // Clicking a thumbnail is an explicit pause + jump (won't auto-resume).
  function onThumb(i: number) {
    userPaused = true
    suspendAutoplay()
    show(i)
  }

  onMounted(() => {
    const leadIdx = ordered.value.findIndex((i) => i.isLead)
    index.value = leadIdx >= 0 ? leadIdx : 0

    // Autoplay only while the card is visible: resume on enter, suspend on exit.
    const obs = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          visible = e.isIntersecting
          if (visible) resumeAutoplay()
          else suspendAutoplay()
        })
      },
      { threshold: 0.4 },
    )
    if (stageRef.value) obs.observe(stageRef.value)
    onBeforeUnmount(() => {
      obs.disconnect()
      if (timer) clearInterval(timer)
      if (revealTimer) clearTimeout(revealTimer)
    })
  })

  return { ordered, index, playing, show, next, toggle, onThumb }
}
