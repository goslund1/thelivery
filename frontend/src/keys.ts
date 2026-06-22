import type { InjectionKey } from 'vue'

// Provided by LiveryCard, bound to that card's livery id. Descendant edit
// components (e.g. EditableText) call it to mark their card as having unsaved
// changes, without having to know the livery id themselves.
export const MarkDirtyKey: InjectionKey<() => void> = Symbol('markCardDirty')
