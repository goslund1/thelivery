# Plan: Split `useUiStore` into focused stores

## Motivation

`useUiStore` currently owns four distinct concerns in ~350 lines:

1. **Theme/text size** — `theme`, `textDelta`, `THEMES`
2. **Edit session** — `isEditing`, dirty tracking, snapshot lifecycle, edit-jump list, exit/save/discard flow
3. **Modal/picker/lightbox state** — login, settings, newCard, exitConfirm, legendConfirm, lightbox, chipPicker, imagePicker, factoidPanel
4. **Filters** — `sectionExpanded`, `upgradesExpanded`, `favoritesOnly`, `disabledCollections`, `allExpanded`

Any component that needs filter state currently imports the whole store and gets the edit session API too. The split makes dependencies readable and each store easier to reason about independently.

## Proposed split

Two extractions, in order of ascending risk:

### Phase 1: `useFilterStore` (low risk)

Extract the expand/collapse and card-visibility concern into `stores/filters.ts`.

**Moves out of `useUiStore`:**
- `allExpanded`, `sectionExpanded`, `upgradesExpanded`
- `favoritesOnly`, `disabledCollections`
- `toggleAll()`, `setSectionExpanded()`, `toggleCollection()`, `isCardVisible()`

**Stays in `useUiStore`:** everything else unchanged.

**Files that need updating (4):**

| File | Currently uses |
|---|---|
| `Filters.vue` | `sectionExpanded`, `setSectionExpanded`, `upgradesExpanded`, `favoritesOnly`, `disabledCollections`, `toggleCollection` |
| `SideBug.vue` | `allExpanded`, `toggleAll` |
| `CollapsibleSection.vue` | `sectionExpanded` |
| `App.vue` | `isCardVisible` |

Each file adds `import { useFilterStore } from '../stores/filters'` and routes the relevant calls there. No other files are touched.

### Phase 2: `useModalStore` (medium risk)

Extract modal/lightbox/picker open states into `stores/modal.ts`.

**Moves out of `useUiStore`:**
- Lightbox: `lightboxSrc`, `lightboxOriginalSrc`, `lightboxImages`, `lightboxIndex`, `openLightbox()`, `closeLightbox()`, `navigateLightbox()`
- ChipPicker: `chipPicker`, `openChipPicker()`, `closeChipPicker()`
- ImagePicker: `imagePicker`, `openImagePicker()`, `openGalleryManager()`, `closeImagePicker()`
- Login modal: `loginOpen`, `openLogin()`, `closeLogin()`, `onLoginSuccess()`
- Settings modal: `settingsOpen`, `openSettings()`, `closeSettings()`
- New card modal: `newCardOpen`, `openNewCard()`, `closeNewCard()`
- Factoid panel: `factoidPanelOpen`, `openFactoidPanel()`, `closeFactoidPanel()`
- Exit confirm: `exitConfirmOpen`, `cancelExit()` (open is set inside `useUiStore.requestExit`; close/confirm stay coordinated — see note)
- Legend confirm: `legendConfirmOpen`, `confirmLegendUpdate()`, `cancelLegendUpdate()`, `requestLegendConfirm()`

**Stays in `useUiStore`:** `isEditing`, dirty tracking, edit-jump list, save/discard/exit flow, `saving`, `handleAuthError`, theme/text size.

**Coordination note:** `exitConfirmOpen` is set inside `requestExit()` which lives in `useUiStore` (because it checks `hasUnsavedChanges`). If it moves to `useModalStore`, `requestExit()` would need to call `useModalStore().openExitConfirm()`. Pinia stores calling each other is fine — `useCardsStore` already does this. Just import lazily (inside the function) to avoid circular init.

**Files that need updating (14):**

| File | Modals used |
|---|---|
| `App.vue` | all close calls on Escape |
| `Lightbox.vue` | lightbox |
| `ChipPicker.vue` | chipPicker |
| `ImagePicker.vue` | imagePicker |
| `TextSection.vue` | openLightbox, openImagePicker |
| `Gallery.vue` | openLightbox |
| `TagCloud.vue` | openChipPicker |
| `CardMeta.vue` | openChipPicker |
| `SubtitleEditor.vue` | openFactoidPanel |
| `FactoidPanel.vue` | factoidPanelOpen, closeFactoidPanel |
| `LoginModal.vue` | loginOpen, closeLogin, onLoginSuccess |
| `UserSettingsModal.vue` | settingsOpen, closeSettings |
| `ExitConfirmModal.vue` | exitConfirmOpen, cancelExit, confirmSaveAndExit, confirmDiscardAndExit |
| `LegendConfirmModal.vue` | legendConfirmOpen, confirmLegendUpdate, cancelLegendUpdate |

Phase 2 is mechanical but touches 14 files. Each one only needs `import { useModalStore }` added and relevant calls rerouted.

## What stays in `useUiStore` after both phases

- `theme`, `textDelta`, `THEMES` — could move to a `usePrefsStore` in a Phase 3, but not worth it yet
- `isEditing`, `dirtyIds`, `hasUnsavedChanges`, `markCardDirty`, `isCardDirty`, `clearCardDirty`, `clearAllDirty`
- `saving`, `handleAuthError`
- `enterEdit`, `requestExit`, `toggleEdit`, `saveCard`, `saveAllDirty`, `confirmSaveAndExit`, `confirmDiscardAndExit`, `cancelExit`
- `_editList`, `editCount`, `currentEditIndex`, `addToEditList`, `saveRange`, `setFocusedEdit`, `getEditAt`
- `legendConfirmOpen` / `requestLegendConfirm` — tightly coupled to `saveAllDirty`; leave here or move with modal store

## Execution order

1. Do Phase 1 first. It's 4 files, self-contained, low risk. Verify with `npm run build` and manual smoke test.
2. Only proceed to Phase 2 after Phase 1 is confirmed working.
3. Run `npm run build` after Phase 2. Smoke test: open/close every modal type, confirm Escape closes all.

## Risk checklist

- [ ] `App.vue` Escape handler closes all modals — after Phase 2 it must import `useModalStore` and call all the close functions from there. Easy to miss one.
- [ ] `requestExit()` sets `exitConfirmOpen` — must call `useModalStore()` from inside the function (lazy, not at store init) to avoid circular dependency.
- [ ] `requestLegendConfirm()` uses a Promise pattern with a stored resolver — keep this in whichever store owns `legendConfirmOpen`.
- [ ] `onLoginSuccess()` calls `enterEdit()` which lives in `useUiStore` — if login moves to `useModalStore`, it must call `useUiStore().enterEdit()` lazily.
- [ ] vue-tsc catches wrong property names but NOT "called a function on the wrong store." After each file change, confirm the build passes before moving to the next file.
