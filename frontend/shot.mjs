import { chromium } from 'playwright'

const url = process.argv[2] || 'http://localhost:5173/'
const out = process.argv[3] || '/tmp/livery-shot.png'
const theme = process.argv[4] // optional: set data-theme

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1200, height: 1600 } })
const errors = []
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
page.on('pageerror', (e) => errors.push('PAGEERROR: ' + e.message))

await page.goto(url, { waitUntil: 'networkidle' })
if (theme) await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme)
await page.waitForSelector('.card', { timeout: 8000 }).catch(() => {})
await page.waitForTimeout(800)

const cards = await page.locator('.card').count()
const titles = await page.locator('.card-title').allInnerTexts()
console.log('cards:', cards)
console.log('titles:', JSON.stringify(titles))
console.log('console errors:', errors.length ? JSON.stringify(errors, null, 2) : 'none')

await page.screenshot({ path: out, fullPage: true })
console.log('screenshot ->', out)
await browser.close()
