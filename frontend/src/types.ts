// The Card data model — mirrors the seed JSON produced by tools/extract and the
// objects served by the Rust API. A card is a generic catalog entry; its content
// is an ordered list of type-dispatched sections, so new section types can be
// added without changing the card schema.

export interface CardImage {
  id: string
  path: string // URL path, e.g. "/uploads/1-0.jpg"
  order: number // the image at order 0 is the lead/feature image
}

// A free-text section with an optional figure (Inspiration, Design Notes, ...).
export interface TextSection {
  type: 'text'
  key: string // stable slug, used for the section filter (e.g. "inspiration")
  label: string
  body: string
  figurePath?: string
}

export interface UpgradeCategory {
  category: string
  parts: string[]
}

export interface Adjustment {
  name: string
  description: string
}

// The Forza tune/build-parts section.
export interface ForzaRecipeSection {
  type: 'forza_recipe'
  key: string
  label: string
  tuneName: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: Adjustment[]
}

export type Section = TextSection | ForzaRecipeSection

export interface Card {
  id: string
  catalogNumber: number
  name: string
  subtitle: string
  isFavorite: boolean
  isLegend: boolean
  collections: string[]
  tags: string[]
  images: CardImage[]
  sections: Section[]
}

export type Theme = 'dark' | 'light' | 'rainbow' | 'clouds' | 'stormy'
