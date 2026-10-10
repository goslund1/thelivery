// Playwright screenshot helper + mobile-pass harness.
//
//   node shot.mjs [url] [out.png] [theme]        one full-page shot (the original use)
//   node shot.mjs --set <label> [url]           viewport set + audits → spec/mobile-pass/<label>/
//   node shot.mjs --compare <labelA> <labelB>   pixel-diff the 1440×900 sets of two labels
//
// --set signs in for the edit-mode view with LIVERY_USER / LIVERY_PASS, or else
// frontend/.shot-login.json (gitignored); without either that view is skipped
// and reported as unverified.
// Needs the backend (:8787) and the dev server (:5173) running. Never saves.
import { chromium } from 'playwright'
import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', 'spec', 'mobile-pass')
const args = process.argv.slice(2)

// ── Original one-shot mode ──────────────────────────────────────────────────

async function runSingle(url, out, theme) {
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
}

// ── Harness ─────────────────────────────────────────────────────────────────

const VIEWPORTS = [
  { w: 360, h: 640, touch: true },
  { w: 390, h: 844, touch: true },
  { w: 844, h: 390, touch: true },
  { w: 820, h: 1180, touch: true },
  { w: 1180, h: 820, touch: true },
  { w: 1440, h: 900, touch: false },
]
const CARD = '#card-1'
const SHOT = { animations: 'disabled', caret: 'hide' }

async function runSet(label, url) {
  if (!label) throw new Error('usage: node shot.mjs --set <label> [url]')
  const outDir = join(ROOT, label)
  // Edit-mode login: env vars, else the gitignored .shot-login.json (a local-only account).
  let user = process.env.LIVERY_USER, pass = process.env.LIVERY_PASS
  const loginFile = join(dirname(fileURLToPath(import.meta.url)), '.shot-login.json')
  if (!(user && pass) && existsSync(loginFile)) ({ user, pass } = JSON.parse(readFileSync(loginFile, 'utf8')))
  let token = null
  if (user && pass) {
    const res = await fetch(new URL('/api/login', url), {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: user, password: pass }),
    })
    if (!res.ok) throw new Error(`login failed: ${res.status}`)
    token = await res.json()
  }

  const browser = await chromium.launch()
  const report = { label, url, when: new Date().toISOString(), editView: token ? 'captured' : 'SKIPPED — no LIVERY_USER/LIVERY_PASS or .shot-login.json (unverified)', viewports: {} }

  // SHOT_ONLY=1440x900 (comma-separated) limits a run to those viewports.
  const only = process.env.SHOT_ONLY?.split(',')
  for (const vp of VIEWPORTS.filter(v => !only || only.includes(`${v.w}x${v.h}`))) {
    const name = `${vp.w}x${vp.h}`
    const dir = join(outDir, name)
    mkdirSync(dir, { recursive: true })
    const ctx = await browser.newContext({
      viewport: { width: vp.w, height: vp.h },
      hasTouch: vp.touch, isMobile: vp.touch, deviceScaleFactor: 1,
      reducedMotion: 'reduce',
    })
    const page = await ctx.newPage()
    const errors = []
    page.on('pageerror', (e) => errors.push(e.message))
    await page.goto(url, { waitUntil: 'networkidle' })
    await page.waitForSelector(CARD, { timeout: 8000 })
    await settle(page)

    const r = { touch: vp.touch, media: await page.evaluate(() => ({
      hoverNone: matchMedia('(hover: none)').matches,
      coarse: matchMedia('(pointer: coarse)').matches,
    })), shots: [], errors }

    await page.evaluate(() => window.scrollTo(0, 0))
    await settle(page)
    await page.screenshot({ ...SHOT, path: join(dir, '1-page-top.png') }); r.shots.push('1-page-top')
    await shootEl(page, vp, CARD, join(dir, '2-card-collapsed.png')); r.shots.push('2-card-collapsed')

    await expandAll(page)
    await shootEl(page, vp, CARD, join(dir, '3-card-expanded.png')); r.shots.push('3-card-expanded')
    await shootEl(page, vp, `${CARD} .ta`, join(dir, '4-tuning.png')); r.shots.push('4-tuning')

    r.overflow = { view: await overflowAudit(page) }
    if (vp.touch) { await assertTouch(page); r.touch = { view: await touchAudit(page) } }
    await page.evaluate(() => window.scrollTo(0, 0))
    r.bug = await bugAudit(page)
    r.flyouts = await flyoutAudit(page, vp.touch)
    const phone = vp.w <= 640
    if (phone) {
      // A6: with an overlay open the bar can't be seen or tapped.
      r.barCovered = {}
      await page.evaluate(() => { window.scrollTo(0, 0); document.querySelector('#card-1 .stage-expand').click() })
      await page.waitForSelector('.lightbox.open')
      await settle(page)
      r.barCovered.lightbox = await barCovered(page)
      await page.evaluate(() => document.querySelector('.lightbox-close').click())
      await page.waitForFunction(() => !document.querySelector('.lightbox.open'))
    }

    if (token) {
      await page.evaluate((t) => {
        localStorage.setItem('auth_token', t.token)
        localStorage.setItem('auth_user', t.username)
        localStorage.setItem('auth_role', t.role)
        localStorage.setItem('auth_must_change', t.mustChangePassword ? '1' : '0')
      }, token)
      await page.reload({ waitUntil: 'networkidle' })
      await page.waitForSelector(CARD)
      if (phone) { await settle(page); r.bugSignedIn = await bugAudit(page) }
      await page.evaluate(() => document.querySelector('.bug-btn[aria-label="Edit mode"]').click())
      await page.waitForSelector('body.editing-mode')
      if (phone) { await settle(page); r.bugEdit = await bugAudit(page) }
      await expandAll(page)
      await shootEl(page, vp, CARD, join(dir, '5-card-edit.png')); r.shots.push('5-card-edit')
      r.overflow.edit = await overflowAudit(page)
      if (vp.touch) { await assertTouch(page); r.touch.edit = await touchAudit(page) }

      // E5: the New Card and Edit Card modals (both a .card inside .nc-overlay).
      const MODAL = '.nc-overlay.open .card'
      for (const [name, open] of [
        ['6-new-card', () => document.querySelector('.tl-btn.tl-green').click()],
        ['7-edit-card', () => document.querySelector('#card-1 .edit-card-btn').click()],
      ]) {
        await page.evaluate(open)
        await page.waitForSelector(MODAL)
        await settle(page)
        await shootEl(page, vp, MODAL, join(dir, `${name}.png`)); r.shots.push(name)
        const view = name.slice(2)
        r.overflow[view] = await overflowAudit(page, MODAL)
        if (vp.touch) { await assertTouch(page); r.touch[view] = await touchAudit(page, '.nc-overlay.open') }
        if (phone) r.barCovered[view] = await barCovered(page)
        await page.evaluate(() => document.querySelector('.nc-overlay.open .nc-close').click())
        await page.waitForFunction(() => !document.querySelector('.nc-overlay.open'))
      }

      // G4: card 1's photo manager, opened from the rail's add/manage panel.
      const MGR = '.image-picker.open .float_imagepicker_panel'
      await page.evaluate(() => document.querySelector('#card-1 .thumb-add-panel').click())
      await page.waitForSelector(MGR)
      await settle(page)
      await shootEl(page, vp, MGR, join(dir, '8-photo-manager.png')); r.shots.push('8-photo-manager')
      r.overflow['photo-manager'] = await overflowAudit(page, MGR)
      if (vp.touch) { await assertTouch(page); r.touch['photo-manager'] = await touchAudit(page, '.image-picker.open') }
      if (phone) r.barCovered['photo-manager'] = await barCovered(page)
      r.photoManager = await page.evaluate((sel) => { const p = document.querySelector(sel).getBoundingClientRect(); return { w: Math.round(p.width), h: Math.round(p.height), columns: getComputedStyle(document.querySelector('.mgr-grid')).gridTemplateColumns.split(' ').length } }, MGR)
      // G7: photo 1's details, from the manager.
      await page.evaluate(() => document.querySelector('.mgr-ctrl-detail').click())
      await page.waitForSelector('.pd-shell')
      await settle(page)
      await shootEl(page, vp, '.pd-shell', join(dir, '9-photo-detail.png')); r.shots.push('9-photo-detail')
      r.overflow['photo-detail'] = await overflowAudit(page, '.pd-shell')
      if (vp.touch) { await assertTouch(page); r.touch['photo-detail'] = await touchAudit(page, '.pd-overlay') }
      if (phone) r.barCovered['photo-detail'] = await barCovered(page)
      await page.evaluate(() => document.querySelector('.pd-close').click())
      await page.waitForFunction(() => !document.querySelector('.pd-shell'))
      await page.evaluate(() => [...document.querySelectorAll('.mgr-footer .mgr-action-btn')].find(b => b.textContent.trim() === 'Done').click())
      await page.waitForFunction(() => !document.querySelector('.image-picker.open'))
      if (phone) {
        r.typing = await typingAudit(page, vp, join(dir, '10-focused-field.png'))
        r.saveCount = await saveCountAudit(page, join(dir, '11-two-unsaved.png'))
      }
      await page.evaluate(() => localStorage.clear())
    }

    report.viewports[name] = r
    await ctx.close()
  }
  await browser.close()

  mkdirSync(outDir, { recursive: true })
  writeFileSync(join(outDir, 'audit.json'), JSON.stringify(report, null, 2))
  const text = formatReport(report)
  writeFileSync(join(outDir, 'audit.txt'), text)
  console.log(text)
}

// Element shot. On a touch viewport an element taller than the screen can't go
// through Playwright's beyond-viewport capture: that drops the touch emulation,
// so (hover: none)/(pointer: coarse) stop matching mid-run and the shot is drawn
// with mouse styling. Grow the viewport to fit the element instead, then restore.
// The mouse viewport keeps the default capture so it stays comparable with before/.
async function shootEl(page, vp, sel, path) {
  const el = page.locator(sel).first()
  const h = await el.evaluate((e) => Math.ceil(e.getBoundingClientRect().height))
  if (!vp.touch || h <= vp.h) return el.screenshot({ ...SHOT, path })
  await page.setViewportSize({ width: vp.w, height: h + 40 })
  await el.evaluate((e) => e.scrollIntoView())
  await settle(page)
  await el.screenshot({ ...SHOT, path })
  await page.setViewportSize({ width: vp.w, height: vp.h })
  await settle(page)
}

// Guard: a touch audit run without touch media would silently pass mouse styling.
async function assertTouch(page) {
  const ok = await page.evaluate(() => matchMedia('(hover: none)').matches && matchMedia('(pointer: coarse)').matches)
  if (!ok) throw new Error('touch emulation lost before a touch audit')
}

async function settle(page) {
  await page.evaluate(() => document.fonts.ready)
  await page.waitForTimeout(600)
}

// Opens every section via the side bug's Expand All (the same path a user takes).
async function expandAll(page) {
  const expanded = await page.evaluate(() => document.querySelector('.bug-toggle-all')?.classList.contains('all-expanded'))
  if (!expanded) await page.evaluate(() => document.querySelector('.bug-toggle-all').click())
  await page.waitForFunction((sel) => [...document.querySelectorAll(`${sel} details.section`)].every(d => d.open), CARD)
  await settle(page)
}

// H3 — sideways document scroll, and anything inside a .card wider than its box
// or reaching past the card's right edge. .thumbs and .ta-tabbar scroll by design.
function overflowAudit(page, root = '.card') {
  return page.evaluate((root) => {
    const path = (el, stop) => {
      const parts = []
      for (let e = el; e && e !== stop; e = e.parentElement) {
        const cls = [...e.classList].filter(c => !/^(active|open|dirty|v-|router)/.test(c)).slice(0, 2)
        parts.unshift(cls.length ? '.' + cls.join('.') : e.tagName.toLowerCase())
      }
      return parts.slice(-4).join(' > ')
    }
    const visible = (el) => {
      const s = getComputedStyle(el)
      if (s.display === 'none' || s.visibility === 'hidden') return false
      const r = el.getBoundingClientRect()
      return r.width > 0 && r.height > 0
    }
    const doc = { scrollWidth: document.documentElement.scrollWidth, innerWidth }
    const found = new Map()
    for (const card of document.querySelectorAll(root)) {
      const cr = card.getBoundingClientRect()
      if (!visible(card)) continue
      for (const el of card.querySelectorAll('*')) {
        if (el.closest('.thumbs, .ta-tabbar') || !visible(el)) continue
        if (el.closest('details:not([open])') && !el.matches('summary, summary *')) continue
        const r = el.getBoundingClientRect()
        // An invisible ::after hit area (T3) may reach past its own box on purpose.
        const a = getComputedStyle(el, '::after')
        const hitArea = a.content !== 'none' && a.position === 'absolute' && parseFloat(a.width) >= 44
        const wide = !hitArea && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0
        const past = r.right > cr.right + 1
        if (!wide && !past) continue
        const key = path(el, card)
        const prev = found.get(key)
        // Name the child that sticks out furthest, so a wide container points at its cause.
        let culprit = null, far = r.right
        if (wide) for (const c of el.children) {
          const cRight = c.getBoundingClientRect().right
          if (cRight > far + 1) { far = cRight; culprit = path(c, el) }
        }
        const entry = { path: key, content: el.scrollWidth, box: el.clientWidth, pastEdgeBy: Math.round(r.right - cr.right), culprit, count: 1 }
        if (prev) { prev.count++; prev.pastEdgeBy = Math.max(prev.pastEdgeBy, entry.pastEdgeBy) } else found.set(key, entry)
      }
    }
    // Keep only the outermost offender: drop entries whose path extends a reported one.
    const items = [...found.values()]
    const kept = items.filter(a => !items.some(b => b !== a && a.path.startsWith(b.path + ' > ')))
    return { sideways: doc.scrollWidth > doc.innerWidth, ...doc, items: kept }
  }, root)
}

// H4 — controls under 44px either way; text fields under 16px.
// Out-of-scope surfaces (Theme Builder, OG Maker, admin) are excluded.
function touchAudit(page, root = 'body') {
  return page.evaluate((root) => {
    const OUT = '.theme-customize-btn, .bug-flyout--builder, [class*="ogm"], [class*="admin"], [class*="og-maker"], .fp-panel'
    const CONTROLS = 'button, a[href], input:not([type=hidden]), select, textarea, summary, [role=button], [contenteditable=true], [tabindex]:not([tabindex="-1"])'
    const FIELDS = 'input:not([type=hidden]):not([type=range]):not([type=checkbox]):not([type=radio]):not([type=file]), select, textarea, [contenteditable=true]'
    const path = (el) => {
      const parts = []
      for (let e = el; e && e !== document.body && parts.length < 3; e = e.parentElement) {
        const cls = [...e.classList].filter(c => !/^(active|open|dirty|v-)/.test(c)).slice(0, 2)
        parts.unshift(cls.length ? '.' + cls.join('.') : e.tagName.toLowerCase())
      }
      return parts.join(' > ')
    }
    // Closed panels here sit at opacity 0 with pointer-events off, or parked off
    // screen. A control's OWN opacity 0 still counts — that's a hover-only control (T2).
    const shown = (el) => {
      for (let e = el; e; e = e.parentElement) {
        const s = getComputedStyle(e)
        if (s.display === 'none' || s.visibility === 'hidden' || s.pointerEvents === 'none') return false
        if (e !== el && +s.opacity === 0) return false
      }
      if (el.closest('details:not([open])') && !el.matches('summary')) return false
      const r = el.getBoundingClientRect()
      if (r.left >= innerWidth || r.right <= 0) return false
      return r.width > 0 && r.height > 0
    }
    // What a finger hits: the box, an absolutely positioned ::after/::before hit
    // area, or for a checkbox/radio the <label> around it.
    const hitSize = (el) => {
      let { width: w, height: h } = el.getBoundingClientRect()
      for (const pseudo of ['::after', '::before']) {
        const ps = getComputedStyle(el, pseudo)
        if (ps.content === 'none' || ps.position !== 'absolute') continue
        w = Math.max(w, parseFloat(ps.width) || 0); h = Math.max(h, parseFloat(ps.height) || 0)
      }
      const label = el.matches('input[type=checkbox], input[type=radio]') && el.closest('label')
      if (label) { const lr = label.getBoundingClientRect(); w = Math.max(w, lr.width); h = Math.max(h, lr.height) }
      return [w, h]
    }
    // Agreed exceptions: inline links in running text
    // (WCAG 2.5.8 inline exception), and editable text, which is tapped to place a caret.
    const EXEMPT = 'p a, [contenteditable=true]'
    // Controls deferred to a later phase are reported apart rather than counted.
    // (Phase 3 cleared the tuning rows; nothing is deferred now.)
    const LATER = ':not(*)'
    const small = new Map(), fonts = new Map(), later = new Map()
    const scope = document.querySelector(root)
    for (const el of scope.querySelectorAll(CONTROLS)) {
      if (el.closest(OUT) || el.disabled || !shown(el) || el.matches(EXEMPT)) continue
      const [hw, hh] = hitSize(el)
      if (hw >= 44 && hh >= 44) continue
      const r = el.getBoundingClientRect()
      const key = path(el)
      if (el.matches(LATER)) { later.set(key, (later.get(key) || 0) + 1); continue }
      const w = Math.round(r.width), h = Math.round(r.height)
      const op = +getComputedStyle(el).opacity
      const prev = small.get(key)
      if (prev) { prev.count++; prev.minW = Math.min(prev.minW, w); prev.minH = Math.min(prev.minH, h) }
      else small.set(key, { path: key, minW: w, minH: h, opacity: op, count: 1 })
    }
    for (const el of scope.querySelectorAll(FIELDS)) {
      if (el.closest(OUT) || !shown(el)) continue
      const px = parseFloat(getComputedStyle(el).fontSize)
      if (px >= 16) continue
      const key = path(el)
      if (el.matches(LATER)) { later.set(key + ' (font)', (later.get(key + ' (font)') || 0) + 1); continue }
      const prev = fonts.get(key)
      if (prev) { prev.count++; prev.minPx = Math.min(prev.minPx, px) } else fonts.set(key, { path: key, minPx: px, count: 1 })
    }
    return { smallTargets: [...small.values()], smallFonts: [...fonts.values()], later: [...later].map(([path, count]) => ({ path, count })) }
  }, root)
}

// L6 — the side bug's size and how many px of it sit over any card. At phone
// width (A1/A7) it is the bottom bar instead: inside the viewport, centred, every
// button 44px, and clear of the tuning panel's sticky header.
async function bugAudit(page) {
  const phone = await page.evaluate(() => innerWidth <= 640)
  if (phone) {
    await page.evaluate(() => { const ta = document.querySelector('#card-1 .ta'); if (ta) window.scrollTo(0, ta.getBoundingClientRect().top + scrollY + 200) })
    await settle(page)
  }
  const out = await page.evaluate((phone) => {
    const bar = document.querySelector('.side-bug'), b = bar.getBoundingClientRect()
    if (phone) {
      const btns = [...bar.querySelectorAll('.bug-btn')].filter(e => e.offsetWidth).map(e => e.getBoundingClientRect())
      let overHeader = 0
      // The header that is stuck at the top (headers scrolling past in flow pass
      // under the bar like any content).
      for (const h of document.querySelectorAll('.ta-header')) {
        const r = h.getBoundingClientRect()
        if (r.top > 1 || r.bottom <= 0) continue
        if (r.bottom <= b.top || r.top >= b.bottom) continue
        overHeader = Math.max(overHeader, Math.min(r.right, b.right) - Math.max(r.left, b.left))
      }
      return {
        phone, width: Math.round(b.width), height: Math.round(b.height), bottomGap: Math.round(innerHeight - b.bottom),
        inside: b.top >= 0 && b.left >= 0 && b.bottom <= innerHeight && b.right <= innerWidth,
        offCentre: Math.round(Math.abs((b.left + b.right) / 2 - innerWidth / 2)),
        buttons: btns.length, minBtn: Math.round(Math.min(...btns.map(r => Math.min(r.width, r.height)))),
        overHeaderPx: Math.max(0, Math.round(overHeader)),
      }
    }
    let overlap = 0
    for (const c of document.querySelectorAll('.card')) {
      const r = c.getBoundingClientRect()
      if (r.bottom <= b.top || r.top >= b.bottom) continue
      overlap = Math.max(overlap, Math.min(r.right, b.right) - Math.max(r.left, b.left))
    }
    return { phone, width: Math.round(b.width), height: Math.round(b.height), overCardPx: Math.max(0, Math.round(overlap)) }
  }, phone)
  if (phone) await page.evaluate(() => window.scrollTo(0, 0))
  return out
}

// A6: is the bar under whatever overlay is open? Nothing of the bar may be on top.
function barCovered(page) {
  return page.evaluate(() => {
    const bar = document.querySelector('.side-bug'), b = bar.getBoundingClientRect()
    const pts = [[b.left + 22, b.top + b.height / 2], [(b.left + b.right) / 2, b.top + b.height / 2], [b.right - 22, b.top + b.height / 2]]
    return pts.every(([x, y]) => { const e = document.elementFromPoint(x, y); return !e || !bar.contains(e) })
  })
}

// A3: two unsaved cards → Save All drawn filled, carrying "2". Types one letter
// into two card titles (never saved; the context is thrown away).
async function saveCountAudit(page, shot) {
  for (const id of ['1', '2']) {
    await page.evaluate((id) => {
      const t = document.querySelector(`#card-${id} .card-title[contenteditable=true]`)
      t.focus(); document.execCommand('insertText', false, 'x'); t.blur()
    }, id)
  }
  await page.waitForTimeout(600)
  await page.evaluate(() => window.scrollTo(0, 0))
  await settle(page)
  await page.screenshot({ ...SHOT, path: shot })
  return page.evaluate(() => {
    const b = document.querySelector('.bug-btn[aria-label="Save all unsaved cards"]')
    return { filled: b.classList.contains('active'), background: getComputedStyle(b).backgroundColor, count: b.querySelector('.bug-sugg-dot')?.textContent ?? null, barOpacity: +getComputedStyle(document.querySelector('.side-bug')).opacity }
  })
}

// A5: focusing a tuning value hides the bar; Enter through three rows never shows
// it (the opacity is sampled every frame); leaving the field brings it back.
async function typingAudit(page, vp, shot) {
  const sel = '#card-1 .ta-val-input'
  const found = await page.evaluate((sel) => {
    const el = [...document.querySelectorAll(sel)].find(e => e.offsetWidth)
    if (!el) return false
    el.scrollIntoView({ block: 'center' }); el.focus(); return true
  }, sel)
  if (!found) return { skipped: 'no visible tuning value field' }
  await page.waitForTimeout(400)
  const op = () => page.evaluate(() => { const s = getComputedStyle(document.querySelector('.side-bug')); return { opacity: +s.opacity, pointerEvents: s.pointerEvents } })
  const focused = await op()
  await page.screenshot({ ...SHOT, path: shot })
  await page.evaluate(() => {
    window.__barMax = 0
    const bar = document.querySelector('.side-bug')
    const tick = () => { window.__barMax = Math.max(window.__barMax, +getComputedStyle(bar).opacity); if (window.__barSampling) requestAnimationFrame(tick) }
    window.__barSampling = true; requestAnimationFrame(tick)
  })
  const hops = []
  for (let i = 0; i < 3; i++) {
    await page.keyboard.press('Enter')
    await page.waitForTimeout(350)
    hops.push(await page.evaluate(() => document.activeElement?.className || document.activeElement?.tagName))
  }
  const maxDuringEnter = await page.evaluate(() => { window.__barSampling = false; return window.__barMax })
  await page.evaluate(() => document.activeElement?.blur())
  await page.waitForTimeout(600)
  const after = await op()
  // An Enter onto a locked row opens a tuning prompt: a dialog the bar must sit
  // under (A6). Check it, then cancel it (past promptReady's 400ms guard).
  let dialog = null
  if (await page.evaluate(() => !!document.querySelector('.ta-prompt-label'))) {
    dialog = { label: await page.evaluate(() => document.querySelector('.ta-prompt-label').textContent.trim()), barCovered: await barCovered(page) }
    await page.evaluate(() => [...document.querySelectorAll('.ta-prompt-btn--muted')].find(b => b.offsetWidth)?.click())
    await page.waitForTimeout(300)
  }
  return { focused, hops, maxDuringEnter, after, dialog }
}

// Opens each side-bug flyout in turn: does it fit the viewport (T6/S1), and on
// touch, what in it is too small to tap (S2/T3).
const FLYOUTS = [
  { name: 'filters', btn: '.bug-btn--filters', panel: '.drawer_menu_panel' },
  { name: 'theme', btn: '.bug-btn[aria-label="Theme"]', panel: '.theme-flyout' },
  { name: 'text-size', btn: '.bug-btn[aria-label="Text size"]', panel: '.text-size-flyout' },
]
async function flyoutAudit(page, touch) {
  const out = {}
  const phone = await page.evaluate(() => innerWidth <= 640)
  if (touch) out['side-bug'] = { fit: { inside: true }, ...(await touchAudit(page, '.side-bug')) }
  for (const f of FLYOUTS) {
    await page.evaluate((sel) => document.querySelector(sel).click(), f.btn)
    await page.waitForSelector(`${f.panel}.open`)
    await page.waitForTimeout(400)
    const fit = await page.evaluate((sel) => {
      const el = document.querySelector(sel), r = el.getBoundingClientRect()
      const bar = document.querySelector('.side-bug').getBoundingClientRect()
      return {
        top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right),
        inside: r.top >= 0 && r.left >= 0 && r.bottom <= innerHeight && r.right <= innerWidth,
        scrollsInside: el.scrollHeight > el.clientHeight,
        // A4: at phone width the flyout opens upward and must stay off the bar.
        gapAboveBar: innerWidth <= 640 ? Math.round(bar.top - r.bottom) : null,
      }
    }, f.panel)
    out[f.name] = { fit, ...(touch ? await touchAudit(page, `${f.panel}.open`) : {}) }
    if (phone) {
      // A4: a tap outside closes it (top-left corner of the page, above the cards).
      await page.touchscreen.tap(4, 4)
      await page.waitForTimeout(300)
      fit.closesOnOutsideTap = await page.evaluate((sel) => !document.querySelector(sel).classList.contains('open'), f.panel)
      if (!fit.closesOnOutsideTap) await page.evaluate((sel) => document.querySelector(sel).click(), f.btn)
    } else {
      await page.evaluate((sel) => document.querySelector(sel).click(), f.btn)
    }
    await page.waitForTimeout(300)
  }
  return out
}

function formatReport(rep) {
  const L = [`mobile-pass set "${rep.label}"  ${rep.when}`, `edit view: ${rep.editView}`, '']
  for (const [name, r] of Object.entries(rep.viewports)) {
    L.push(`══ ${name} ${r.touch ? 'touch' : 'mouse'}  (hover:none=${r.media.hoverNone} coarse=${r.media.coarse})  shots: ${r.shots.length}`)
    if (r.errors.length) L.push(`  page errors: ${r.errors.join(' | ')}`)
    for (const [view, o] of Object.entries(r.overflow)) {
      L.push(`  overflow [${view}]: document ${o.sideways ? `SCROLLS SIDEWAYS (${o.scrollWidth} > ${o.innerWidth})` : 'ok'}; ${o.items.length} element(s)`)
      for (const i of o.items) L.push(`    ${i.path}   content ${i.content} in box ${i.box}${i.pastEdgeBy > 1 ? `, ${i.pastEdgeBy}px past card edge` : ''}${i.culprit ? `  ← ${i.culprit}` : ''}${i.count > 1 ? `  ×${i.count}` : ''}`)
    }
    for (const [view, t] of Object.entries(r.touch || {})) {
      L.push(`  touch [${view}]: ${t.smallTargets.length} target(s) under 44px, ${t.smallFonts.length} field(s) under 16px`)
      for (const s of t.smallTargets) L.push(`    ${s.minW}×${s.minH}  ${s.path}${s.opacity < 1 ? `  (opacity ${s.opacity})` : ''}${s.count > 1 ? `  ×${s.count}` : ''}`)
      for (const f of t.smallFonts) L.push(`    font ${f.minPx}px  ${f.path}${f.count > 1 ? `  ×${f.count}` : ''}`)
      if (t.later?.length) L.push(`    (Phase 3, not counted: ${t.later.map(x => `${x.path.split(' > ').pop()} ×${x.count}`).join(', ')})`)
    }
    if (r.photoManager) L.push(`  photo manager: ${r.photoManager.w}×${r.photoManager.h}, ${r.photoManager.columns} grid column(s)`)
    const g = r.bug
    if (g.phone) L.push(`  bottom bar: ${g.width}×${g.height}, ${g.inside ? 'inside viewport' : 'OFF SCREEN'}, ${g.bottomGap}px above the bottom, ${g.offCentre ? `${g.offCentre}px OFF CENTRE` : 'centred'}; ${g.buttons} buttons, smallest ${g.minBtn}px; ${g.overHeaderPx ? `${g.overHeaderPx}px OVER the stuck .ta-header` : 'clear of the stuck .ta-header'}`)
    else L.push(`  side bug: ${g.width}×${g.height}, ${g.overCardPx ? `${g.overCardPx}px over a card` : 'clear of cards'} (page top, as loaded)`)
    for (const [k, b] of [['signed in', r.bugSignedIn], ['editing', r.bugEdit]]) if (b) L.push(`  bottom bar (${k}): ${b.width}×${b.height}, ${b.inside ? 'inside viewport' : 'OFF SCREEN'}, ${b.offCentre ? `${b.offCentre}px OFF CENTRE` : 'centred'}; ${b.buttons} buttons, smallest ${b.minBtn}px; ${b.overHeaderPx ? `${b.overHeaderPx}px OVER the stuck .ta-header` : 'clear of the stuck .ta-header'}`)
    if (r.saveCount) L.push(`  two unsaved cards: Save ${r.saveCount.filled ? 'filled' : 'NOT FILLED'} (${r.saveCount.background}), count ${r.saveCount.count}, bar opacity ${r.saveCount.barOpacity}`)
    if (r.barCovered) L.push(`  bar under overlays: ${Object.entries(r.barCovered).map(([k, v]) => `${k} ${v ? 'yes' : 'NO — BAR SHOWS'}`).join(', ')}`)
    if (r.typing) {
      const t = r.typing
      if (t.skipped) L.push(`  typing: skipped (${t.skipped})`)
      else L.push(`  typing: focused → bar opacity ${t.focused.opacity}, pointer-events ${t.focused.pointerEvents}; Enter ×3 (${t.hops.join(' → ')}) → max opacity ${t.maxDuringEnter}; blurred → opacity ${t.after.opacity}`)
      if (t.dialog) L.push(`  bar under the "${t.dialog.label}" prompt: ${t.dialog.barCovered ? 'yes' : 'NO — BAR SHOWS'}`)
    }
    for (const [name, f] of Object.entries(r.flyouts)) {
      const { fit } = f
      L.push(`  flyout ${name}: ${fit.inside ? 'inside viewport' : `OFF SCREEN (top ${fit.top}, bottom ${fit.bottom}, left ${fit.left}, right ${fit.right})`}${fit.scrollsInside ? ', scrolls inside' : ''}${fit.gapAboveBar != null ? `, ${fit.gapAboveBar >= 0 ? `${fit.gapAboveBar}px above the bar` : `OVERLAPS THE BAR by ${-fit.gapAboveBar}px`}` : ''}${fit.closesOnOutsideTap != null ? `, ${fit.closesOnOutsideTap ? 'closes on outside tap' : 'DOES NOT CLOSE on outside tap'}` : ''}`)
      for (const s of f.smallTargets || []) L.push(`    ${s.minW}×${s.minH}  ${s.path}${s.count > 1 ? `  ×${s.count}` : ''}`)
      for (const x of f.smallFonts || []) L.push(`    font ${x.minPx}px  ${x.path}${x.count > 1 ? `  ×${x.count}` : ''}`)
    }
    L.push('')
  }
  return L.join('\n')
}

// ── Desktop parity ──────────────────────────────────────────────────────────

async function runCompare(a, b) {
  if (!a || !b) throw new Error('usage: node shot.mjs --compare <labelA> <labelB>')
  const vp = '1440x900'
  const dirA = join(ROOT, a, vp), dirB = join(ROOT, b, vp)
  const diffDir = join(ROOT, b, `diff-vs-${a}`)
  const browser = await chromium.launch()
  const page = await browser.newPage()
  let bad = 0
  for (const f of readdirSync(dirA).filter(f => f.endsWith('.png')).sort()) {
    const fb = join(dirB, f)
    if (!existsSync(fb)) { console.log(`${f}: MISSING in ${b}`); bad++; continue }
    const res = await page.evaluate(async ([ua, ub]) => {
      const load = (src) => new Promise((ok, no) => { const i = new Image(); i.onload = () => ok(i); i.onerror = no; i.src = src })
      const [ia, ib] = await Promise.all([load(ua), load(ub)])
      const w = Math.max(ia.width, ib.width), h = Math.max(ia.height, ib.height)
      const grab = (img) => { const c = new OffscreenCanvas(w, h); const x = c.getContext('2d'); x.drawImage(img, 0, 0); return x.getImageData(0, 0, w, h).data }
      const da = grab(ia), db = grab(ib)
      const out = new OffscreenCanvas(w, h), ox = out.getContext('2d')
      ox.drawImage(ib, 0, 0); ox.fillStyle = 'rgba(255,255,255,0.7)'; ox.fillRect(0, 0, w, h)
      const od = ox.getImageData(0, 0, w, h)
      let n = 0
      for (let p = 0; p < da.length; p += 4) {
        if (da[p] !== db[p] || da[p + 1] !== db[p + 1] || da[p + 2] !== db[p + 2] || da[p + 3] !== db[p + 3]) {
          n++; od.data[p] = 255; od.data[p + 1] = 0; od.data[p + 2] = 0; od.data[p + 3] = 255
        }
      }
      let png = null
      if (n) { ox.putImageData(od, 0, 0); const blob = await out.convertToBlob(); png = [...new Uint8Array(await blob.arrayBuffer())] }
      return { n, sizeA: `${ia.width}×${ia.height}`, sizeB: `${ib.width}×${ib.height}`, png }
    }, [dataUrl(join(dirA, f)), dataUrl(fb)])
    const size = res.sizeA === res.sizeB ? res.sizeA : `${res.sizeA} → ${res.sizeB}`
    if (res.n) {
      bad++
      mkdirSync(diffDir, { recursive: true })
      writeFileSync(join(diffDir, f), Buffer.from(res.png))
      console.log(`${f}: ${res.n} pixels differ (${size}) → ${join(diffDir, f)}`)
    } else console.log(`${f}: identical (${size})`)
  }
  await browser.close()
  console.log(bad ? `\n${bad} view(s) differ at ${vp}` : `\nall ${vp} views identical`)
  process.exitCode = bad ? 1 : 0
}

function dataUrl(p) {
  return 'data:image/png;base64,' + readFileSync(p).toString('base64')
}

// ── Entry (last, so the consts above are initialised) ─────────────────────

if (args[0] === '--set') await runSet(args[1], args[2] || 'http://localhost:5173/')
else if (args[0] === '--compare') await runCompare(args[1], args[2])
else await runSingle(args[0] || 'http://localhost:5173/', args[1] || '/tmp/livery-shot.png', args[2])
