/**
 * Forza car list scraper
 *
 * FH6: forzahorizoncar.com/en/cars/index.html  — data-* attributes on .cc-card elements
 * FH5: forza.fandom.com/wiki/Forza_Horizon_5/Cars — wikitable with name+PI columns
 *
 * Usage:
 *   npm run scrape          — scrape both games
 *   npm run scrape:fh5      — FH5 only
 *   npm run scrape:fh6      — FH6 only (re-run periodically as FH6 adds cars)
 *
 * Output: ../../backend/seed/cars.json (additive upsert — safe to re-run)
 */

import { load } from 'cheerio'
import { readFileSync, writeFileSync, existsSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, resolve } from 'path'

const __dir = dirname(fileURLToPath(import.meta.url))
const OUT   = resolve(__dir, '../../backend/seed/cars.json')

const MULTI_MAKES = ['Aston Martin', 'Land Rover', 'Alfa Romeo', 'De Tomaso', 'Shelby']

function slugify(str) {
  return str.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/-+/g, '-').replace(/-$/, '')
}

function titleCase(str) {
  return str.split(' ').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')
}

// ── FH6 ─────────────────────────────────────────────────────────────────────

async function scrapeFH6() {
  console.log('Fetching FH6 car list…')
  const res = await fetch('https://forzahorizoncar.com/en/cars/index.html')
  if (!res.ok) throw new Error(`FH6 fetch failed: ${res.status}`)
  const html = await res.text()
  const $ = load(html)

  const cars = []
  $('.cc-card').each((_, el) => {
    const c = $(el)
    const search    = c.attr('data-search') || ''
    const makeRaw   = c.attr('data-make') || ''
    const makeWords = makeRaw.toLowerCase().split(/\s+/)

    // Extract model: tokens after slug+make words, up to the year token
    const tokens = search.split(/\s+/)
    let i = 1 + makeWords.length
    const modelParts = []
    while (i < tokens.length && !tokens[i].match(/^(19|20)\d{2}$/)) {
      modelParts.push(tokens[i])
      i++
    }
    const year  = tokens[i] ? parseInt(tokens[i]) : null
    const model = titleCase(modelParts.join(' '))
    const slug  = c.attr('data-slug') || slugify(`${makeRaw}-${model}`)

    cars.push({
      id:       `fh6-${slug}`,
      game:     'FH6',
      make:     makeRaw,
      model,
      year,
      class:    c.attr('data-class') || null,
      category: c.attr('data-cat')   || null,
      drive:    c.attr('data-drive') || null,
      country:  c.attr('data-country') || null,
      decade:   c.attr('data-decade') || null,
      status:   c.attr('data-status') || null,
      pi:       null,
      dlc:      null,
    })
  })

  console.log(`  FH6: ${cars.length} cars`)
  return cars
}

// ── FH5 ─────────────────────────────────────────────────────────────────────

async function scrapeFH5() {
  console.log('Fetching FH5 car list…')
  // Use MediaWiki API — bypasses the bot-detection that blocks direct page loads
  const apiUrl = 'https://forza.fandom.com/api.php?action=parse&page=Forza_Horizon_5%2FCars&prop=text&format=json'
  const res = await fetch(apiUrl, {
    headers: { 'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36' }
  })
  if (!res.ok) throw new Error(`FH5 fetch failed: ${res.status}`)
  const json = await res.json()
  const html = json?.parse?.text?.['*'] ?? ''
  const $ = load(html)

  // Main car list — first sortable table in the API response (API uses .table.sortable, not .wikitable)
  const tables = $('table.sortable')
  const table  = tables.eq(0)

  const cars = []
  table.find('tbody tr').slice(1).each((_, row) => {
    const cells = $(row).find('td').map((_, td) => $(td).text().trim()).get()
    if (!cells.length) return

    const raw    = cells[0]
    const piRaw  = cells[cells.length - 1]

    const yearMatch = raw.match(/(19|20)\d{2}/)
    const year      = yearMatch ? parseInt(yearMatch[0]) : null
    const carName   = year ? raw.slice(0, raw.search(/(19|20)\d{2}/)).trim() : raw
    const afterYear = year ? raw.slice(raw.search(/(19|20)\d{2}/) + 4).trim() : ''
    const dlcMatch  = afterYear.match(/DLC:\s*([^)]+)/)
    const dlc       = dlcMatch ? dlcMatch[1].trim() : null

    const multi = MULTI_MAKES.find(m => carName.startsWith(m))
    let make = '', model = carName
    if (multi) {
      make  = multi
      model = carName.slice(multi.length).trim()
    } else {
      const parts = carName.split(' ')
      make  = parts[0]
      model = parts.slice(1).join(' ')
    }

    const piClass = piRaw.match(/^([A-Z])/)?.[1] || null
    const pi      = parseInt(piRaw.replace(/^[A-Z]/, '')) || null
    const slug    = slugify(`${make}-${model}`)

    cars.push({
      id:       `fh5-${slug}`,
      game:     'FH5',
      make,
      model,
      year,
      class:    piClass,
      category: null,
      drive:    null,
      country:  null,
      decade:   year ? `${Math.floor(year / 10) * 10}s` : null,
      status:   'confirmed',
      pi,
      dlc,
    })
  })

  console.log(`  FH5: ${cars.length} cars`)
  return cars
}

// ── Merge + upsert ───────────────────────────────────────────────────────────

function merge(existing, incoming) {
  const map = new Map(existing.map(c => [c.id, c]))
  let added = 0, updated = 0
  for (const car of incoming) {
    if (map.has(car.id)) {
      map.set(car.id, { ...map.get(car.id), ...car })
      updated++
    } else {
      map.set(car.id, car)
      added++
    }
  }
  console.log(`  +${added} new, ~${updated} updated`)
  return Array.from(map.values()).sort((a, b) =>
    a.game.localeCompare(b.game) || a.make.localeCompare(b.make) || (a.year ?? 0) - (b.year ?? 0)
  )
}

// ── Main ─────────────────────────────────────────────────────────────────────

const args = process.argv.slice(2)
const game = args.find(a => a.startsWith('--game='))?.split('=')[1]
       || (args.includes('--game') ? args[args.indexOf('--game') + 1] : null)

const existing = existsSync(OUT) ? JSON.parse(readFileSync(OUT, 'utf8')) : []

const incoming = []
if (!game || game === 'FH6') incoming.push(...await scrapeFH6())
if (!game || game === 'FH5') incoming.push(...await scrapeFH5())

const result = merge(existing, incoming)
writeFileSync(OUT, JSON.stringify(result, null, 2))
console.log(`\nWrote ${result.length} total cars → ${OUT}`)
