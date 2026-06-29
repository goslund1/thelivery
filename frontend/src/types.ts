// The Card data model — mirrors the seed JSON produced by tools/extract and the
// objects served by the Rust API. A card is a generic catalog entry; its content
// is an ordered list of type-dispatched sections, so new section types can be
// added without changing the card schema.

export interface CardImage {
  id: string
  path: string      // original full-res URL, e.g. "/uploads/uuid.jpg"
  thumbPath?: string // 200px-wide JPEG for the thumb rail
  stagePath?: string // 1000px-wide JPEG for the slideshow stage
  order: number     // the image at order 0 is the lead/feature image
  included?: boolean // undefined or true = in slideshow; false = in pool but hidden
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

export interface AdjustmentRow {
  tab: string     // 'tires' | 'alignment' | 'arb' | 'springs' | 'damping' | 'aero' | 'brakes' | 'differential'
  group: string   // e.g. 'Tire Pressure', 'Camber'
  key: string     // unique slug, e.g. 'tiresFront'
  label: string   // e.g. 'Front', 'Rear'
  unit: string    // e.g. '°', '%', '' for unitless
  min: number
  max: number
  stock: number
  value: number
  step: number
}

// The Forza tune/build-parts section.
export interface ForzaRecipeSection {
  type: 'forza_recipe'
  key: string
  label: string
  tuneName: string
  shareCode: string
  showStock?: boolean
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
}

export type Section = TextSection | ForzaRecipeSection

export interface Card {
  id: string
  catalogNumber: number
  name: string
  subtitle: string
  liveryShareCode?: string
  isFavorite: boolean
  isLegend: boolean
  collections: string[]
  tags: string[]
  images: CardImage[]
  sections: Section[]
  colors?: Record<string, string>  // per-field color overrides, e.g. { name: '#ff0000' }
}

export type Theme = 'dark' | 'light' | 'rainbow' | 'clouds' | 'stormy'
