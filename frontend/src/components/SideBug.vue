<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useFilterStore } from '../stores/filters'
import { useCardsStore } from '../stores/cards'
import { useAuthStore } from '../stores/auth'
import { onClickOutside } from '../composables/onClickOutside'
import { hideTip, refreshTip } from '../composables/tooltip'
import type { Theme } from '../types'
import ThemeBuilder from './ThemeBuilder.vue'

const ui = useUiStore()
const modal = useModalStore()
const filters = useFilterStore()
const store = useCardsStore()
const auth = useAuthStore()

// Inline SVGs for each theme (matches the original theme-flyout icons).
const themeIcons: Record<Theme, string> = {
  dark: `<svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>`,
  light: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>`,
  rainbow: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none"><path d="M3 17a9 9 0 0 1 18 0" stroke="#e83d9c" stroke-width="2" stroke-linecap="round"></path><path d="M6 17a6 6 0 0 1 12 0" stroke="#3dc7e8" stroke-width="2" stroke-linecap="round"></path><path d="M9 17a3 3 0 0 1 6 0" stroke="#f2c14e" stroke-width="2" stroke-linecap="round"></path></svg>`,
  clouds: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M6.5 19a4.5 4.5 0 0 1-.5-8.97 5 5 0 0 1 9.78-1.44A4 4 0 0 1 17.5 19h-11z"></path></svg>`,
  stormy: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none"><path d="M6.5 14a4.5 4.5 0 0 1-.5-8.97 5 5 0 0 1 9.78-1.44A4 4 0 0 1 17.5 14h-11z" fill="currentColor"></path><path d="M13 14l-2.5 4h2.5l-2 4.5" stroke="#e8c84a" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" fill="none"></path></svg>`,
}
const themeLabels: Record<Theme, string> = {
  dark: 'Dark', light: 'Light', rainbow: 'Rainbow', clouds: 'Clouds', stormy: 'Stormy',
}
const deltas = [5, 4, 3, 2, 1, 0, -1, -2, -3]
function deltaLabel(d: number) {
  return d > 0 ? `+${d}` : d < 0 ? `−${Math.abs(d)}` : '0'
}

type FlyoutName = 'theme' | 'text' | 'menu' | null
const open = ref<FlyoutName>(null)
const themeBuilderOpen = ref(false)
const bugRef = ref<HTMLElement | null>(null)
const sideBugEl = ref<HTMLElement | null>(null)
const hamburgerBtn = ref<HTMLElement | null>(null)
const flyoutStyle = ref<Record<string, string>>({})

onClickOutside(bugRef, () => (open.value = null))

const themeIcon = computed(() => themeIcons[ui.theme])

async function toggle(name: Exclude<FlyoutName, null>, anchor: HTMLElement) {
  hideTip() // Rule A: opening a flyout dismisses the tooltip immediately
  if (open.value === name) { open.value = null; return }
  open.value = name
  await nextTick()
  // Place the flyout just left of the side bug, vertically aligned to the button.
  const bug = sideBugEl.value!.getBoundingClientRect()
  const a = anchor.getBoundingClientRect()
  const flyoutRight = window.innerWidth - bug.left + 8
  flyoutStyle.value = {
    top: `${Math.max(12, a.top)}px`,
    right: `${flyoutRight}px`,
    maxWidth: `${window.innerWidth - flyoutRight - 8}px`,
  }
}

// Anchor the bug's LEFT edge to the catalog column's right edge (so on wide
// screens it hugs the content instead of floating at the window edge), and line
// the hamburger up with the page title's top. Mirrors the original
// positionSideBug(), with the same defensive viewport clamps.
function positionSideBug() {
  const bug = sideBugEl.value
  const ham = hamburgerBtn.value
  const catalog = document.querySelector('.catalog')
  const title = document.querySelector('.page-head h1')
  if (!bug || !ham || !catalog || !title) return

  const catRect = catalog.getBoundingClientRect()
  const bugWidth = bug.offsetWidth
  const rightVal = Math.max(0, window.innerWidth - catRect.right - bugWidth)
  bug.style.right = `${rightVal}px`

  // Keep any open flyout or theme builder anchored to the new bug position
  if (Object.keys(flyoutStyle.value).length > 0) {
    const flyoutRight = bugWidth + rightVal + 8
    flyoutStyle.value = {
      ...flyoutStyle.value,
      right: `${flyoutRight}px`,
      maxWidth: `${window.innerWidth - flyoutRight - 8}px`,
    }
  }

  const titleTop = title.getBoundingClientRect().top
  const hamOffset = ham.getBoundingClientRect().top - bug.getBoundingClientRect().top
  const bugHeight = bug.offsetHeight
  const rawTop = titleTop - hamOffset
  bug.style.top = `${Math.max(12, Math.min(rawTop, window.innerHeight - bugHeight - 12))}px`
}

onMounted(() => {
  nextTick(positionSideBug)
  window.addEventListener('resize', positionSideBug)
})
onBeforeUnmount(() => window.removeEventListener('resize', positionSideBug))
// Reposition once cards have loaded (catalog column width can change).
watch(() => store.cards.length, () => nextTick(positionSideBug))

function pickTheme(t: Theme) { ui.theme = t; open.value = null }
function openThemeBuilder() { open.value = null; themeBuilderOpen.value = true }
function pickDelta(d: number) { ui.textDelta = d; open.value = null }

function nearestVisibleCard(): Element | null {
  return Array.from(document.querySelectorAll('.card')).reduce<Element | null>(
    (best, c) => {
      const r = c.getBoundingClientRect()
      if (r.bottom <= 0 || r.top >= window.innerHeight) return best
      if (!best) return c
      return Math.abs(r.top) < Math.abs(best.getBoundingClientRect().top) ? c : best
    }, null)
}

// Edit-mode key opens the edit bar / exit-confirm modal → hide tooltip (Rule A).
// Anchors scroll so the nearest card doesn't jump when edit affordances reflow.
function onEditClick() {
  hideTip()
  const anchor = nearestVisibleCard()
  const before = anchor?.getBoundingClientRect().top ?? null
  ui.toggleEdit()
  nextTick(() => {
    if (anchor && before !== null) window.scrollBy(0, anchor.getBoundingClientRect().top - before)
  })
}
// Expand/collapse-all — two distinct scroll behaviours:
// • Expanding: just preserve the current viewport position (anchor pattern).
// • Collapsing: snap the card that owned the content at the viewport top to the
//   top of the window. elementFromPoint finds which card the user was looking at;
//   we hold a reference and scrollTo its post-collapse document position (measured
//   after nextTick because other sections above it may also have collapsed).
function onToggleAll() {
  const willCollapse = filters.allExpanded

  if (willCollapse) {
    const el = document.elementFromPoint(window.innerWidth / 2, 1)
    const card = el?.closest('.card') as HTMLElement | null

    filters.toggleAll()

    nextTick(() => {
      if (card) window.scrollTo({ top: card.getBoundingClientRect().top + window.scrollY })
      refreshTip('Expand All Sections')
    })
  } else {
    const anchor = nearestVisibleCard()
    const before = anchor?.getBoundingClientRect().top ?? null

    filters.toggleAll()

    nextTick(() => {
      if (anchor && before !== null) window.scrollBy(0, anchor.getBoundingClientRect().top - before)
      refreshTip('Collapse All Sections')
    })
  }
}
</script>

<template>
  <div ref="bugRef">
    <div class="side-bug" ref="sideBugEl">
      <button v-if="auth.isAuthenticated" class="bug-btn" aria-label="Account settings" v-tip="'Account settings'" @click="modal.openSettings()">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="8" r="4"></circle>
          <path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"></path>
        </svg>
      </button>
      <button v-if="auth.isAuthenticated" class="bug-btn" :class="{ active: ui.isEditing }" aria-label="Edit mode" v-tip="() => ui.isEditing ? 'Exit edit mode' : 'Enter edit mode'" @click="onEditClick">
        <svg viewBox="0 0 24 24" width="34" height="34" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <g transform="rotate(225 12 12)">
            <path d="M12 5 L7 8 L10.5 11.5 L10.5 21"></path>
            <path d="M12 5 L17 8 L13.5 11.5 L14.2 13.5 L12.5 15 L14 16.5 L12.2 18 L13.7 19.5 L13.5 21"></path>
            <circle cx="12" cy="8" r="0.65" fill="currentColor" stroke="none"></circle>
          </g>
        </svg>
      </button>
      <button ref="hamburgerBtn" class="bug-btn" aria-label="Section menu" v-tip="() => open === 'menu' ? 'Close section filters' : 'Open section filters'" @click="(e) => toggle('menu', e.currentTarget as HTMLElement)">
        <span class="bug-lines"><span></span><span></span><span></span></span>
      </button>
      <button class="bug-btn" aria-label="Theme" v-tip="() => 'Theme: ' + themeLabels[ui.theme]" @click="(e) => toggle('theme', e.currentTarget as HTMLElement)">
        <span class="theme-icon-slot" v-html="themeIcon"></span>
      </button>
      <button class="bug-btn" aria-label="Text size" v-tip="'Text Size'" @click="(e) => toggle('text', e.currentTarget as HTMLElement)">
        <svg class="bug-text-icon-svg" viewBox="0 0 24 24" width="18" height="18" fill="none">
          <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="2"></circle>
          <line x1="15" y1="15" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"></line>
          <text x="10" y="13.5" text-anchor="middle" font-size="9" font-family="Oswald, sans-serif" font-weight="600" fill="currentColor">T</text>
        </svg>
      </button>
      <button class="bug-btn bug-toggle-all" :class="{ 'all-expanded': filters.allExpanded }" aria-label="Expand or collapse all" v-tip="() => filters.allExpanded ? 'Collapse All Sections' : 'Expand All Sections'" @click="onToggleAll">
        <span class="bug-tri"></span>
      </button>
    </div>

    <!-- Theme flyout -->
    <div class="bug-flyout theme-flyout" :class="{ open: open === 'theme' }" :style="flyoutStyle" v-scroll-contain>
      <button v-for="t in ui.THEMES" :key="t" class="theme-option" :class="{ active: ui.theme === t }" @click="pickTheme(t)">
        <span v-html="themeIcons[t]"></span> {{ themeLabels[t] }}
      </button>
      <button class="theme-option theme-customize-btn" @click="openThemeBuilder">⚙ Customize</button>
    </div>

    <!-- Theme builder panel -->
    <div v-if="themeBuilderOpen" class="bug-flyout bug-flyout--builder open" :style="flyoutStyle">
      <ThemeBuilder @close="themeBuilderOpen = false" />
    </div>

    <!-- Text-size flyout -->
    <div class="bug-flyout text-size-flyout" :class="{ open: open === 'text' }" :style="flyoutStyle" v-scroll-contain>
      <button v-for="d in deltas" :key="d" class="text-size-option" :class="{ active: ui.textDelta === d }" @click="pickDelta(d)">
        {{ deltaLabel(d) }}
      </button>
    </div>

    <!-- Menu flyout (filters) -->
    <div class="bug-flyout" :class="{ open: open === 'menu' }" :style="flyoutStyle" v-scroll-contain>
      <slot name="menu" />
    </div>
  </div>
</template>

<style scoped>
.theme-customize-btn {
  margin-top: 4px;
  border-top: 1px solid var(--panel-edge);
  color: var(--steel) !important;
  font-size: 11px !important;
}
.theme-customize-btn:hover {
  color: var(--gold) !important;
}
.bug-flyout--builder {
  padding: 0 !important;
  min-width: 0 !important;
  background: transparent !important;
  border: none !important;
  box-shadow: none !important;
}
</style>
