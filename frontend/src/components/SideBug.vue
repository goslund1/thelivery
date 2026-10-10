<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useUiStore } from '../stores/ui'
import { useModalStore } from '../stores/modal'
import { useFilterStore } from '../stores/filters'
import { useCardsStore } from '../stores/cards'
import { useAuthStore } from '../stores/auth'
import { useThemeStore } from '../stores/theme'
import { onClickOutside } from '../composables/onClickOutside'
import { hideTip, refreshTip } from '../composables/tooltip'
import type { Theme } from '../types'
import ThemeBuilder from './ThemeBuilder.vue'
import { useEditCycle } from '../composables/editCycle'

const ui = useUiStore()
const modal = useModalStore()
const filters = useFilterStore()
const store = useCardsStore()
const auth = useAuthStore()
const themeStore = useThemeStore()
const { onCycleDirty } = useEditCycle()

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

const bugRef = ref<HTMLElement | null>(null)
const sideBugEl = ref<HTMLElement | null>(null)
const hamburgerBtn = ref<HTMLElement | null>(null)
const flyoutStyle = ref<Record<string, string>>({})

// Phone width (≤640px): the bug is a floating bar at the bottom of the screen
// (A1). Its flyouts open upward from it (A4). Wider screens keep the side bug.
const phoneQuery = window.matchMedia('(max-width: 640px)')
const isPhone = ref(phoneQuery.matches)
function onPhoneChange() {
  isPhone.value = phoneQuery.matches
  open.value = null
  flyoutStyle.value = {}
  nextTick(positionSideBug)
}

onClickOutside(bugRef, () => { open.value = null })

// A5: while a text field has focus the phone bar and its flyouts are hidden, never
// moved. Hopping field to field (Next, the tuning focus proxy) keeps them hidden;
// they come back a moment after focus leaves the last field. Fields inside the
// bug's own flyouts don't count. The class has rules only at phone width.
const TEXT_FIELD = 'input:not([type=range], [type=checkbox], [type=radio], [type=file]), textarea, [contenteditable=true]'
const typing = ref(false)
let typingTimer: number | undefined
function isTextField(el: EventTarget | null): boolean {
  return el instanceof Element && el.matches(TEXT_FIELD) && !bugRef.value?.contains(el)
}
function onFocusIn(e: FocusEvent) {
  if (!isTextField(e.target)) return
  clearTimeout(typingTimer)
  typing.value = true
}
function onFocusOut(e: FocusEvent) {
  if (!isTextField(e.target) || isTextField(e.relatedTarget)) return
  clearTimeout(typingTimer)
  typingTimer = window.setTimeout(() => {
    if (!isTextField(document.activeElement)) typing.value = false
  }, 250)
}

const themeIcon = computed(() => themeIcons[ui.theme])

async function toggle(name: Exclude<FlyoutName, null>, anchor: HTMLElement) {
  hideTip() // Rule A: opening a flyout dismisses the tooltip immediately
  if (open.value === name) { open.value = null; return }
  open.value = name
  await nextTick()
  const bug = sideBugEl.value!.getBoundingClientRect()
  const a = anchor.getBoundingClientRect()
  if (isPhone.value) { placePhoneFlyout(name, bug, a); return }
  // Place the flyout just left of the side bug, vertically aligned to the button.
  const flyoutRight = window.innerWidth - bug.left + 8
  const top = Math.max(12, a.top)
  flyoutStyle.value = {
    top: `${top}px`,
    right: `${flyoutRight}px`,
    maxWidth: `${window.innerWidth - flyoutRight - 8}px`,
    // Height from the space below the anchor, not the whole viewport, so a tall
    // flyout scrolls inside instead of running off the bottom.
    maxHeight: `${window.innerHeight - top - 12}px`,
  }
}

// A4: at phone width a flyout opens upward, 8px above the bar, centred on its
// button and kept 8px inside the screen edges. Measured from the bar's own rect,
// so it is right whether or not Safari's toolbar is showing.
const flyoutEls = ref<Record<string, HTMLElement | null>>({})
function placePhoneFlyout(name: Exclude<FlyoutName, null>, bar: DOMRect, a: DOMRect) {
  const margin = 8
  const maxWidth = window.innerWidth - 2 * margin
  const width = Math.min(flyoutEls.value[name]?.offsetWidth ?? 0, maxWidth)
  const left = Math.max(margin, Math.min(a.left + a.width / 2 - width / 2, window.innerWidth - width - margin))
  flyoutStyle.value = {
    bottom: `${window.innerHeight - bar.top + margin}px`,
    left: `${left}px`,
    maxWidth: `${maxWidth}px`,
    maxHeight: `${bar.top - 2 * margin}px`,
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
  // Phone: the bar is placed by CSS alone; drop what a wider layout set inline.
  if (isPhone.value) {
    bug.style.top = ''
    bug.style.right = ''
    return
  }

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
      maxHeight: `${window.innerHeight - parseFloat(flyoutStyle.value.top ?? '12') - 12}px`,
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
  phoneQuery.addEventListener('change', onPhoneChange)
  document.addEventListener('focusin', onFocusIn)
  document.addEventListener('focusout', onFocusOut)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', positionSideBug)
  phoneQuery.removeEventListener('change', onPhoneChange)
  document.removeEventListener('focusin', onFocusIn)
  document.removeEventListener('focusout', onFocusOut)
  clearTimeout(typingTimer)
})
// Reposition once cards have loaded (catalog column width can change).
watch(() => store.cards.length, () => nextTick(positionSideBug))

function pickTheme(t: Theme) {
  // Go through the theme store so the inline color variables set by
  // theme.load()/ThemeBuilder are reset too — setting ui.theme alone only
  // flips data-theme, which inline styles on <html> override.
  if (themeStore.current) themeStore.setAmbiance(t)
  else ui.theme = t
  open.value = null
}
function openThemeBuilder() { open.value = null; modal.openThemeBuilder() }
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
  <div ref="bugRef" :class="{ 'bug-root--typing': typing }">
    <div class="side-bug" :class="{ 'side-bug--editing': ui.isEditing }" ref="sideBugEl">
      <button v-if="auth.isAuthenticated" class="bug-btn bug-btn--view-only" aria-label="Account settings" v-tip="'Account settings'" @click="modal.openSettings()">
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
      <!-- Phone edit controls (A2): the bottom dock and pill are hidden at ≤640px and
           their actions live in the bar. Exit is the edit button above. -->
      <template v-if="ui.isEditing">
        <button class="bug-btn bug-btn--phone-edit" :class="{ active: ui.hasUnsavedChanges }" aria-label="Save all unsaved cards" :disabled="!ui.hasUnsavedChanges || ui.saving" @click="ui.saveAllDirty()">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 3h11l3 3v15H5z"></path><path d="M8 3v5h7V3"></path><rect x="8" y="13" width="8" height="6"></rect>
          </svg>
          <span v-if="ui.dirtyIds.size" class="bug-sugg-dot">{{ ui.dirtyIds.size }}</span>
        </button>
        <button class="bug-btn bug-btn--phone-edit" aria-label="Jump to next edit" :disabled="!ui.editCount" @click="onCycleDirty">
          <svg viewBox="0 0 256 512" width="10" height="20" fill="currentColor"><path d="M.1 29.3C-1.4 47 11.7 62.4 29.3 63.9l8 .7C70.5 67.3 96 95 96 128.3L96 224l-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 95.7c0 33.3-25.5 61-58.7 63.8l-8 .7C11.7 449.6-1.4 465 .1 482.7s16.9 30.7 34.5 29.2l8-.7c34.1-2.8 64.2-18.9 85.4-42.9c21.2 24 51.2 40 85.4 42.9l8 .7c17.6 1.5 33.1-11.6 34.5-29.2s-11.6-33.1-29.2-34.5l-8-.7C185.5 444.7 160 417 160 383.7l0-95.7 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-95.7c0-33.3 25.5-61 58.7-63.8l8-.7c17.6-1.5 30.7-16.9 29.2-34.5S239-1.4 221.3 .1l-8 .7C179.2 3.6 149.2 19.7 128 43.7c-21.2-24-51.2-40-85.4-42.9l-8-.7C17-1.4 1.6 11.7 .1 29.3z"/></svg>
        </button>
        <button class="bug-btn bug-btn--phone-edit" aria-label="New card" @click="modal.openNewCard()">
          <svg viewBox="0 0 20 20" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M 9 2 L 2 2 L 2 18 L 18 18 L 18 9"/><line x1="10" y1="5.5" x2="17" y2="5.5"/><line x1="13.5" y1="2.5" x2="13.5" y2="8.5"/>
          </svg>
        </button>
      </template>
      <button ref="hamburgerBtn" class="bug-btn bug-btn--filters" aria-label="Section menu" v-tip="() => open === 'menu' ? 'Close section filters' : 'Open section filters'" @click="(e) => toggle('menu', e.currentTarget as HTMLElement)">
        <span class="bug-lines"><span></span><span></span><span></span></span>
        <span v-if="auth.isAuthenticated && modal.pendingSuggestionCount" class="bug-sugg-dot">{{ modal.pendingSuggestionCount }}</span>
      </button>
      <button class="bug-btn bug-btn--view-only" aria-label="Theme" v-tip="() => 'Theme: ' + themeLabels[ui.theme]" @click="(e) => toggle('theme', e.currentTarget as HTMLElement)">
        <span class="theme-icon-slot" v-html="themeIcon"></span>
      </button>
      <button class="bug-btn bug-btn--view-only" aria-label="Text size" v-tip="'Text Size'" @click="(e) => toggle('text', e.currentTarget as HTMLElement)">
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
    <div :ref="(el) => { flyoutEls.theme = el as HTMLElement | null }" class="bug-flyout theme-flyout drawer_theme_panel" :class="{ open: open === 'theme' }" :style="flyoutStyle">
      <button v-for="t in ui.THEMES" :key="t" class="theme-option" :class="{ active: ui.theme === t }" @click="pickTheme(t)">
        <span v-html="themeIcons[t]"></span> {{ themeLabels[t] }}
      </button>
      <button class="theme-option theme-customize-btn" @click="openThemeBuilder">⚙ Customize</button>
    </div>

    <!-- Theme builder panel -->
    <div v-if="modal.themeBuilderOpen" class="bug-flyout bug-flyout--builder open" :style="flyoutStyle">
      <ThemeBuilder @close="modal.closeThemeBuilder()" />
    </div>

    <!-- Text-size flyout -->
    <div :ref="(el) => { flyoutEls.text = el as HTMLElement | null }" class="bug-flyout text-size-flyout drawer_text_panel" :class="{ open: open === 'text' }" :style="flyoutStyle">
      <button v-for="d in deltas" :key="d" class="text-size-option" :class="{ active: ui.textDelta === d }" @click="pickDelta(d)">
        {{ deltaLabel(d) }}
      </button>
    </div>

    <!-- Menu flyout (filters) -->
    <div :ref="(el) => { flyoutEls.menu = el as HTMLElement | null }" class="bug-flyout drawer_menu_panel" :class="{ open: open === 'menu' }" :style="flyoutStyle">
      <slot name="menu" />
    </div>
  </div>
</template>

<style scoped>
.bug-btn--phone-edit {
  display: none;
  position: relative;
}
.bug-btn--phone-edit:disabled { opacity: 0.35; cursor: default; }

/* Phones (A1–A6): a floating bar at the bottom, in the edit pill's language
   (EditBar.vue). Viewing shows Account, Edit, Filters, Theme, Text size, Expand;
   editing swaps Account, Theme and Text size for Save All, Next edit, New Card. */
@media (max-width: 640px) {
  .side-bug {
    top: auto;
    right: auto;
    left: 50%;
    bottom: calc(10px + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    flex-direction: row;
    padding: 0 6px;
    background: color-mix(in srgb, var(--panel) 80%, transparent);
    border: 1px solid var(--accent);
    border-radius: 999px;
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    box-shadow: 0 2px 16px rgba(0,0,0,0.4);
  }
  .side-bug .bug-btn {
    width: 44px;
    height: 44px;
    border-bottom: none;
    border-radius: 999px;
  }
  .bug-btn--phone-edit { display: flex; }
  .side-bug--editing .bug-btn--view-only { display: none; }
  /* A3: Save All is drawn filled while anything is unsaved; its count inverts. */
  .bug-btn.active .bug-sugg-dot {
    background: var(--panel);
    color: var(--accent);
  }
  .bug-flyout {
    transform: translateY(8px);
  }
  .bug-flyout.open {
    transform: translateY(0);
  }
  /* A5: hidden while a text field has focus; never moved. */
  .side-bug { transition: opacity 0.15s ease; }   /* flyouts keep catalog.css's .2s */
  .bug-root--typing .side-bug,
  .bug-root--typing .bug-flyout.open {
    opacity: 0;
    pointer-events: none;
  }
}

.bug-btn--filters {
  position: relative;
}
.bug-sugg-dot {
  position: absolute;
  top: 4px;
  right: 4px;
  background: var(--accent);
  color: var(--bg);
  border-radius: 8px;
  font-size: 9px;
  font-family: 'JetBrains Mono', monospace;
  font-weight: bold;
  padding: 0 4px;
  line-height: 14px;
  pointer-events: none;
}

.theme-customize-btn {
  margin-top: 4px;
  border-top: 1px solid var(--panel-edge);
  color: var(--muted) !important;
  font-size: 11px !important;
}
.theme-customize-btn:hover {
  color: var(--accent) !important;
}
.bug-flyout--builder {
  padding: 0 !important;
  min-width: 0 !important;
  background: transparent !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  border: none !important;
  box-shadow: none !important;
}

/* S3: the Theme Builder is desktop-only; theme switching stays everywhere. */
@media (max-width: 640px), (hover: none) {
  .theme-customize-btn { display: none; }
}
</style>
