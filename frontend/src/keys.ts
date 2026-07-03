import type { InjectionKey } from 'vue'

// Provided by CardView, bound to that card's id. Descendant edit components
// (e.g. EditableText) call it to mark their card as having unsaved changes,
// without having to know the card id themselves.
export const MarkDirtyKey: InjectionKey<() => void> = Symbol('markCardDirty')

// Provided by CardView so descendants can read the card id without crawling the DOM.
export const CardIdKey: InjectionKey<string> = Symbol('cardId')
