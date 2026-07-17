// The Card data model — mirrors the seed JSON produced by tools/extract and the
// objects served by the Rust API. A card is a generic catalog entry; its content
// is an ordered list of type-dispatched sections, so new section types can be
// added without changing the card schema.

export interface CardImage {
  id: number         // images table PK — the stable identity; paths live only in the DB
  path: string      // original full-res URL, e.g. "/uploads/uuid.jpg" — resolved server-side
  thumbPath?: string // 200px-wide JPEG for the thumb rail
  stagePath?: string // 1000px-wide JPEG for the slideshow stage
  order: number     // the image at order 0 is the lead/feature image
  included?: boolean  // undefined or true = in slideshow; false = in pool but hidden
  imageRole?: string  // 'gallery' (default) | 'refimg' — determines naming and default inclusion
  carId?: string      // links this photo to a specific car in the registry
  liveryId?: number  // FK into liveries table; set once photo is tagged with a livery
  alt?: string       // descriptive alt text / caption; used for SEO and accessibility
}

// A free-text section with an optional figure (Inspiration, Design Notes, ...).
export interface TextSection {
  type: 'text'
  key: string // stable slug, used for the section filter (e.g. "inspiration")
  label: string
  body: string
  figurePath?: string
  defaultOpen?: boolean // undefined = follow global toggle; false = start collapsed
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

// Tune data within a single car slot.
export interface CardTune {
  tuneName: string
  tuneType?: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
  isSuggested?: boolean
  pendingPresetId?: number
}

// One car slot in a multi-car card. Each car has 1+ tunes (TuneTabs, Step 4).
export interface CardCar {
  carId: string
  carName?: string
  liveryId?: number
  liveryName?: string
  tunes: CardTune[]
}

// Deprecated — replaced by CardCar + CardTune. Kept until all variants[] are migrated.
export interface CardVariant {
  liveryId?: number
  tuneId?: number
  carId: string
  carName?: string
  liveryName?: string
  tuneName: string
  tuneType?: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
  isSuggested?: boolean
  pendingPresetId?: number
}
export type CarVariant = CardVariant

// The Forza tune/build-parts section.
export interface ForzaRecipeSection {
  type: 'forza_recipe'
  key: string
  label: string
  tuneName: string
  shareCode: string
  showStock?: boolean
  defaultOpen?: boolean // undefined = follow global toggle; false = start collapsed
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
  cars?: CardCar[]      // multi-car or multi-tune cards (replaces variants[])
  variants?: CardVariant[]  // deprecated — migrated to cars[] at startup
}

// ── Identity entity types (match API responses from /api/liveries, /api/tunes) ──

export interface TuneType {
  id: number
  name: string
  sortOrder: number
}

export interface Livery {
  id: number
  carId: string
  serial: string     // e.g. 'FH6-NISRVGTSP99-L001'
  name: string
  isFactory: boolean
  carColorId: number | null
  shareCode: string | null
  colorPrimary: string | null
  colorSecondary: string | null
  createdAt: string
}

export interface Tune {
  id: number
  liveryId: number
  carId: string
  serial: string       // e.g. 'FH6-NISRVGTSP99-L001-T001'
  officialName: string | null
  typeId: number | null
  shareCode: string | null
  coreSpecs: string | null   // JSON string → parse to Record<string,string>
  upgrades: string | null    // JSON string → parse to UpgradeCategory[]
  adjustments: string | null // JSON string → parse to AdjustmentRow[]
  createdAt: string
}

export type Section = TextSection | ForzaRecipeSection

export type OgTextStyle = 'POSTCARD' | 'SIGNAL' | 'GHOST'

export type OgFont =
  | 'BEBAS_NEUE'
  | 'OSWALD'
  | 'CINZEL'
  | 'BLACK_OPS_ONE'
  | 'ANTON'
  | 'RACING_SANS_ONE'
  | 'ORBITRON'
  | 'GRADUATE'
  | 'RUSSO_ONE'
  | 'BARLOW_CONDENSED'
  | 'AUDIOWIDE'
  | 'BIG_SHOULDERS_DISPLAY'

export interface OgTextBox {
  style: OgTextStyle
  font?: OgFont
  content: string
  x: number; y: number; w: number; h: number
  rotateDeg: number
  shearX: number
}

export interface OgConfig {
  photoId: number
  logoVisible: boolean
  textBoxes: OgTextBox[]
}

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
  colors?: Record<string, string>
  accentOverride?: string  // CSS color; overrides --accent on this card only
  carId?: string  // FK into the cars registry; null for multi-car/showcase cards
  shareOverlayConfig?: OgConfig  // saved OG Maker output; drives /share/:id/card.png
}

export interface Car {
  id: string       // e.g. fh6-nissan-skyline-gtr-r34
  game: 'FH5' | 'FH6'
  make: string
  model: string
  year: number | null
  class: string | null    // D/C/B/A/S1/S2/X
  pi: number | null       // stock PI (FH5 only)
  drive: string | null    // FWD/RWD/AWD (FH6 only)
  country: string | null
  category: string | null // e.g. Modern Sports (FH6 only)
  decade: string | null
  status: string | null
  dlc: string | null      // null = base game; pack name = paid DLC
  code: string | null     // serial segment e.g. 'NISRVGTSP99'; unique per game
}

export type Theme = 'dark' | 'light' | 'rainbow' | 'clouds' | 'stormy'
