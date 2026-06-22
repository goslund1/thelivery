// One-time extraction: parse the original single-file HTML catalog into
//   - backend/seed/liveries.json   (the Livery[] seed data)
//   - backend/uploads/*.png        (decoded base64 images)
// Run with: npm run extract
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as cheerio from 'cheerio';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '../..');
// The original single-file app, retired to archive/ after the Vue migration.
const HTML = resolve(repoRoot, 'archive/livery_catalog_edited.html');
const UPLOADS = resolve(repoRoot, 'backend/uploads');
const SEED = resolve(repoRoot, 'backend/seed/liveries.json');

mkdirSync(UPLOADS, { recursive: true });
mkdirSync(dirname(SEED), { recursive: true });

const html = readFileSync(HTML, 'utf8');
const $ = cheerio.load(html);

// Text of an element excluding any descendant <button> (chips embed a remove button).
function chipText(el) {
  const c = $(el).clone();
  c.find('button').remove();
  return c.text().trim();
}
function txt(el) {
  return $(el).text().trim();
}
// Decode a data:image/...;base64,XXXX URI -> Buffer, or null if not a data URI.
function decodeDataUri(src) {
  if (!src || !src.startsWith('data:image')) return null;
  const comma = src.indexOf(',');
  return Buffer.from(src.slice(comma + 1), 'base64');
}
// Sniff the real image type from magic bytes (the data URIs claim png but the
// bytes are often jpeg). Returns a file extension.
function sniffExt(buf) {
  if (buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) return 'jpg';
  if (buf[0] === 0x89 && buf[1] === 0x50 && buf[2] === 0x4e && buf[3] === 0x47) return 'png';
  if (buf[0] === 0x52 && buf[1] === 0x49 && buf[2] === 0x46 && buf[3] === 0x46) return 'webp';
  if (buf[0] === 0x47 && buf[1] === 0x49 && buf[2] === 0x46) return 'gif';
  return 'bin';
}
// Save image buffer using a base name; the real extension is sniffed. Returns URL path.
function saveImage(buf, baseName) {
  const filename = `${baseName}.${sniffExt(buf)}`;
  writeFileSync(resolve(UPLOADS, filename), buf);
  return `/uploads/${filename}`;
}

const liveries = [];

$('.card').each((_, cardEl) => {
  const $card = $(cardEl);
  const isLegend = $card.hasClass('legend-card');
  const id = $card.find('.stage').attr('data-group'); // "1".."6","legend"

  // catalog number from "NO. 001"
  const numText = $card.find('.card-number span').first().text(); // "NO. 001"
  const catalogNumber = parseInt((numText.match(/(\d+)/) || [])[1] || '0', 10);

  const name = txt($card.find('.card-title').first());
  const subtitle = txt($card.find('.card-sub').first());
  const isFavorite = $card.find('.fav-star').hasClass('favorited');
  const collections = ($card.attr('data-collections') || '')
    .split(',').map(s => s.trim()).filter(Boolean);
  const tags = $card.find('.tag-cloud .tag.chip').map((_, e) => chipText(e)).get();

  // Gallery images come from the stage; data-index = order, .active = lead.
  const images = [];
  $card.find('.stage img').each((_, img) => {
    const order = parseInt($(img).attr('data-index') || '0', 10);
    const isLead = $(img).hasClass('active');
    const buf = decodeDataUri($(img).attr('src'));
    if (!buf) return;
    const path = saveImage(buf, `${id}-${order}`);
    images.push({ id: `${id}-${order}`, path, isLead, order });
  });
  images.sort((a, b) => a.order - b.order);

  // Section helper: body text + optional gutter figure image.
  function readSection(sectionName, textSelector) {
    const $s = $card.find(`details[data-section="${sectionName}"]`);
    const body = txt($s.find(textSelector).first());
    let figurePath;
    const figImg = $s.find('.gutter-figure img').first();
    if (figImg.length) {
      const buf = decodeDataUri(figImg.attr('src'));
      if (buf) figurePath = saveImage(buf, `${id}-${sectionName}`);
    }
    return figurePath ? { body, figurePath } : { body };
  }

  const inspiration = readSection('inspiration', '.anecdote-text, .gutter-text');
  const designNotes = readSection('notes', '.gutter-text, .anecdote-text');

  // Recipe / tune
  const $recipe = $card.find('details[data-section="recipe"]');
  const tuneName = txt($recipe.find('.tune-name').first());
  const shareCode = txt($recipe.find('.plate b').first());

  const keys = $recipe.find('.recipe-table tr').first().find('th').map((_, e) => txt(e)).get();
  const vals = $recipe.find('.recipe-table tr').eq(1).find('td').map((_, e) => txt(e)).get();
  const coreSpecs = {};
  keys.forEach((k, i) => { coreSpecs[k] = vals[i] || ''; });

  const upgrades = $recipe.find('.kit-body .kit-cat').map((_, cat) => ({
    category: txt($(cat).find('.kit-cat-label').first()),
    parts: $(cat).find('.kit-list li').map((_, li) => txt(li)).get(),
  })).get();

  const adjustments = $recipe.find('.recipe-adjustments li').map((_, li) => ({
    name: txt($(li).find('b').first()),
    description: txt($(li).find('span').first()),
  })).get();

  liveries.push({
    id, catalogNumber, name, subtitle, isFavorite, isLegend,
    collections, tags, images,
    inspiration, designNotes,
    recipe: { tuneName, shareCode, coreSpecs, upgrades, adjustments },
  });
});

writeFileSync(SEED, JSON.stringify(liveries, null, 2));

// Report
const imgTotal = liveries.reduce((n, l) => n + l.images.length, 0);
console.log(`Wrote ${liveries.length} liveries -> ${SEED}`);
console.log(`Decoded ${imgTotal} gallery images -> ${UPLOADS}`);
for (const l of liveries) {
  console.log(`  [${l.id}] ${l.name}  (#${l.catalogNumber}) imgs=${l.images.length} tags=${l.tags.length} upgrades=${l.recipe.upgrades.length} adj=${l.recipe.adjustments.length}${l.isLegend ? '  (LEGEND)' : ''}`);
}
