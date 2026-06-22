// The Livery data model — mirrors the seed JSON produced by tools/extract and the
// objects served by the Rust API. This is the schema the original HTML encoded
// implicitly through the DOM.

export interface LiveryImage {
  id: string
  path: string // URL path, e.g. "/uploads/1-0.jpg"
  isLead: boolean
  order: number
}

export interface SectionContent {
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

export interface Recipe {
  tuneName: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: Adjustment[]
}

export interface Livery {
  id: string
  catalogNumber: number
  name: string
  subtitle: string
  isFavorite: boolean
  isLegend: boolean
  collections: string[]
  tags: string[]
  images: LiveryImage[]
  inspiration: SectionContent
  designNotes: SectionContent
  recipe: Recipe
}

export type Theme = 'dark' | 'light' | 'rainbow' | 'clouds' | 'stormy'
