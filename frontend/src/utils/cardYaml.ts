import * as yaml from 'js-yaml'
import type { Card, Section, TextSection, ForzaRecipeSection, AdjustmentRow, UpgradeCategory } from '../types'

export function cardToYaml(card: Card): string {
  const doc: Record<string, unknown> = {
    name: card.name,
    subtitle: card.subtitle,
    collections: [...card.collections],
    tags: [...card.tags],
    sections: card.sections.map(sectionToDoc),
  }

  const imageNote = card.images.length > 0
    ? `# ${card.images.length} image${card.images.length !== 1 ? 's' : ''} on original card — add via card editor after importing`
    : '# No images on original card'

  const header = [
    '# Thelivery card export',
    '# Import via Settings → Migrate → Import Card',
    imageNote,
    '',
  ].join('\n')

  return header + yaml.dump(doc, { lineWidth: 120, indent: 2 })
}

function sectionToDoc(s: Section): Record<string, unknown> {
  if (s.type === 'text') {
    const d: Record<string, unknown> = { type: 'text', key: s.key, label: s.label, body: s.body }
    if (s.figurePath) d.figurePath = s.figurePath
    return d
  }
  return {
    type: 'forza_recipe',
    key: s.key,
    label: s.label,
    tuneName: s.tuneName,
    shareCode: s.shareCode,
    specs: { ...s.coreSpecs },
    upgrades: s.upgrades.map(u => ({ category: u.category, parts: [...u.parts] })),
    adjustments: s.adjustments.map(a => ({
      tab: a.tab, group: a.group, label: a.label,
      unit: a.unit, min: a.min, max: a.max,
      stock: a.stock, value: a.value, step: a.step,
    })),
  }
}

export type ParseResult =
  | { ok: true; card: Omit<Card, 'id' | 'catalogNumber'> }
  | { ok: false; error: string }

export function yamlToCard(text: string): ParseResult {
  let doc: unknown
  try {
    doc = yaml.load(text)
  } catch (e: any) {
    return { ok: false, error: `YAML parse error: ${e.message}` }
  }

  if (!doc || typeof doc !== 'object' || Array.isArray(doc)) {
    return { ok: false, error: 'Invalid YAML: expected an object at the top level' }
  }
  const d = doc as Record<string, unknown>

  if (!d.name || typeof d.name !== 'string') {
    return { ok: false, error: 'Missing required field: name' }
  }

  const sections: Section[] = []
  if (Array.isArray(d.sections)) {
    for (const s of d.sections) {
      const parsed = parseSection(s)
      if (parsed) sections.push(parsed)
    }
  }

  return {
    ok: true,
    card: {
      name: d.name,
      subtitle: typeof d.subtitle === 'string' ? d.subtitle : '',
      isFavorite: false,
      isLegend: false,
      collections: Array.isArray(d.collections) ? d.collections.map(String) : [],
      tags: Array.isArray(d.tags) ? d.tags.map(String) : [],
      images: [],
      sections,
    },
  }
}

function parseSection(s: unknown): Section | null {
  if (!s || typeof s !== 'object' || Array.isArray(s)) return null
  const d = s as Record<string, unknown>

  if (d.type === 'text') {
    return {
      type: 'text',
      key: String(d.key ?? 'text'),
      label: String(d.label ?? 'Section'),
      body: String(d.body ?? ''),
      ...(typeof d.figurePath === 'string' ? { figurePath: d.figurePath } : {}),
    } satisfies TextSection
  }

  if (d.type === 'forza_recipe') {
    const specs: Record<string, string> = {}
    if (d.specs && typeof d.specs === 'object' && !Array.isArray(d.specs)) {
      for (const [k, v] of Object.entries(d.specs as Record<string, unknown>)) {
        specs[k] = String(v ?? '')
      }
    }

    const upgrades: UpgradeCategory[] = Array.isArray(d.upgrades)
      ? (d.upgrades as unknown[]).filter(u => u && typeof u === 'object').map((u: any) => ({
          category: String(u.category ?? ''),
          parts: Array.isArray(u.parts) ? (u.parts as unknown[]).map(String) : [],
        }))
      : []

    const adjustments: AdjustmentRow[] = Array.isArray(d.adjustments)
      ? (d.adjustments as unknown[]).filter(a => a && typeof a === 'object').map((a: any) => ({
          tab: String(a.tab ?? ''),
          group: String(a.group ?? ''),
          key: String(a.key ?? `${a.tab ?? ''}${String(a.group ?? '').replace(/\s/g, '')}${String(a.label ?? '').replace(/\s/g, '')}`),
          label: String(a.label ?? ''),
          unit: String(a.unit ?? ''),
          min: Number(a.min ?? 0),
          max: Number(a.max ?? 0),
          stock: Number(a.stock ?? 0),
          value: Number(a.value ?? 0),
          step: Number(a.step ?? 1),
        }))
      : []

    return {
      type: 'forza_recipe',
      key: String(d.key ?? 'recipe'),
      label: String(d.label ?? 'Tune & Build'),
      tuneName: String(d.tuneName ?? ''),
      shareCode: String(d.shareCode ?? ''),
      coreSpecs: specs,
      upgrades,
      adjustments,
    } satisfies ForzaRecipeSection
  }

  return null
}
