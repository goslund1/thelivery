#!/usr/bin/env node
// Generates `code` values for every car in backend/seed/cars.json.
// Rules:
//   code = makePrefix (3 chars) + modelCode (up to 6 chars), uppercase, alphanumeric only
//   Scoped per game — uniqueness is (game, code), not global.
//   Collision resolution: append 2-digit year → append sequential digit (2,3…)
//   After 9 collision attempts: flag for manual review, skip.
//
// Output: writes backend/seed/cars.json in-place with a `code` field added.
//         prints a summary of collisions resolved and any manual-review flags.

import { readFileSync, writeFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __dir = dirname(fileURLToPath(import.meta.url))
const SEED  = join(__dir, '../../backend/seed/cars.json')

const cars = JSON.parse(readFileSync(SEED, 'utf8'))

// ── Make prefix ───────────────────────────────────────────────────────────────
function makePrefix(make) {
  // Strip non-alpha, take first 3 uppercase chars
  return make.replace(/[^a-zA-Z]/g, '').toUpperCase().slice(0, 3)
}

// ── Model code ────────────────────────────────────────────────────────────────
function modelCode(model, make) {
  // Remove make name if it appears at the start of the model field
  let m = model ?? ''
  const makePat = new RegExp('^' + make.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\s*', 'i')
  m = m.replace(makePat, '')

  // Remove parentheticals and "Forza Edition" / "FE" / "Le Mans" qualifiers
  m = m.replace(/\(.*?\)/g, '')
  m = m.replace(/\b(forza edition|forza|edition|le mans|coupe|sedan|wagon|convertible|roadster|spider|spyder|cabriolet|targa|touring|sport|series)\b/gi, '')
  m = m.trim()

  // Extract all alphanumeric tokens
  const tokens = m.match(/[a-zA-Z0-9]+/g) ?? []

  // Strategy: prefer tokens that are short and contain digits (model codes like R34, 911, GT40)
  // Score: tokens with digits score higher; shorter scores higher
  const scored = tokens.map(t => ({
    t,
    score: (/\d/.test(t) ? 10 : 0) + Math.max(0, 6 - t.length),
  })).sort((a, b) => b.score - a.score)

  // Build code: take top tokens until we have 6 chars
  let code = ''
  for (const { t } of scored) {
    if (code.length >= 6) break
    code += t.toUpperCase().slice(0, 6 - code.length)
  }

  // Fallback: if empty (e.g. model was blank), use first 6 of raw model alphanum
  if (!code) {
    code = (model ?? '').replace(/[^a-zA-Z0-9]/g, '').toUpperCase().slice(0, 6)
  }

  return code || 'UNKNWN'
}

// ── Collision resolution ───────────────────────────────────────────────────────
// Map: game → Set of codes already assigned
const assigned = { FH5: new Map(), FH6: new Map() } // code → carId

function resolve(game, candidate, year, carId) {
  const scope = assigned[game] ?? (assigned[game] = new Map())

  // 1. Try candidate as-is
  if (!scope.has(candidate)) {
    scope.set(candidate, carId)
    return { code: candidate, flagged: false }
  }

  // 2. Append 2-digit year
  const yr = String(year ?? '').slice(-2).padStart(2, '0')
  const withYear = candidate + yr
  if (!scope.has(withYear)) {
    scope.set(withYear, carId)
    return { code: withYear, flagged: false, resolved: 'year' }
  }

  // 3. Append sequential digit
  for (let d = 2; d <= 9; d++) {
    const withDigit = withYear + d
    if (!scope.has(withDigit)) {
      scope.set(withDigit, carId)
      return { code: withDigit, flagged: false, resolved: `year+${d}` }
    }
  }

  // 4. Flag for manual review
  return { code: null, flagged: true }
}

// ── Main ──────────────────────────────────────────────────────────────────────
let resolved = 0
let flagged = []

for (const car of cars) {
  if (car.code) continue // already set (re-run safe)

  const mp   = makePrefix(car.make)
  const mc   = modelCode(car.model, car.make)
  const cand = mp + mc

  const result = resolve(car.game, cand, car.year, car.id)

  if (result.flagged) {
    flagged.push({ id: car.id, game: car.game, make: car.make, model: car.model, candidate: cand })
    car.code = null
  } else {
    car.code = result.code
    if (result.resolved) resolved++
  }
}

writeFileSync(SEED, JSON.stringify(cars, null, 2) + '\n', 'utf8')

console.log(`\nCar code generation complete`)
console.log(`  Total cars:        ${cars.length}`)
console.log(`  Collision-resolved:${resolved}`)
console.log(`  Manual review:     ${flagged.length}`)

if (flagged.length) {
  console.log(`\nManual review required (set car.code in seed/cars.json):`)
  for (const f of flagged) {
    console.log(`  ${f.game} | ${f.make} ${f.model} (${f.id}) — candidate was ${f.candidate}`)
  }
}
