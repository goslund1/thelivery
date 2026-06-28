import { ref, watch } from 'vue'

export interface FactoidType {
  key: string
  name: string
  options: string[]
}

const STORAGE_KEY = 'factoid_schema'

const DEFAULTS: FactoidType[] = [
  { key: 'make_model', name: 'Make / Model', options: [] },
  { key: 'technique', name: 'Technique',     options: [] },
  { key: 'style',     name: 'Style',         options: [] },
  { key: 'rims',      name: 'Rims',          options: [] },
]

function load(): FactoidType[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw) as FactoidType[]
  } catch { /* ignore */ }
  return DEFAULTS.map(d => ({ ...d, options: [...d.options] }))
}

function save(schema: FactoidType[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(schema))
}

// Module singleton — shared across all component instances
const schema = ref<FactoidType[]>(load())

watch(schema, (v) => save(v), { deep: true })

export function useFactoidSchema() {
  function addType(name: string) {
    const key = name.toLowerCase().replace(/[^a-z0-9]+/g, '_') + '_' + Date.now()
    schema.value = [...schema.value, { key, name, options: [] }]
  }

  function removeType(key: string) {
    schema.value = schema.value.filter(t => t.key !== key)
  }

  function renameType(key: string, name: string) {
    schema.value = schema.value.map(t => t.key === key ? { ...t, name } : t)
  }

  function addOption(key: string, option: string) {
    const trimmed = option.trim()
    if (!trimmed) return
    schema.value = schema.value.map(t => {
      if (t.key !== key) return t
      if (t.options.includes(trimmed)) return t
      return { ...t, options: [...t.options, trimmed] }
    })
  }

  function removeOption(key: string, option: string) {
    schema.value = schema.value.map(t =>
      t.key === key ? { ...t, options: t.options.filter(o => o !== option) } : t
    )
  }

  function moveType(key: string, dir: -1 | 1) {
    const idx = schema.value.findIndex(t => t.key === key)
    if (idx < 0) return
    const next = idx + dir
    if (next < 0 || next >= schema.value.length) return
    const copy = [...schema.value]
    ;[copy[idx], copy[next]] = [copy[next], copy[idx]]
    schema.value = copy
  }

  function optionsFor(position: number, cardValues: string[]): string[] {
    const type = schema.value[position]
    const schemaOpts = type?.options ?? []
    const cardVal = cardValues[position] ?? ''
    const all = [...new Set([...schemaOpts, ...cardValues.filter((_, i) => i === position && cardVal)])]
    return all.filter(Boolean).sort()
  }

  return { schema, addType, removeType, renameType, addOption, removeOption, moveType, optionsFor }
}
