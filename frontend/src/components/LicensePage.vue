<script setup lang="ts">
import { onMounted } from 'vue'

// /license — the human-readable side of the attribution that GET /api/cards
// already gives scrapers. Rendered by App.vue in place of the catalog (there is
// no client-side router; App reads location.pathname, like /ignition).
onMounted(() => { document.title = 'License — The Livery' })
</script>

<template>
  <article class="page-head license-page">
    <p class="eyebrow">The Livery</p>
    <h1>Builds, Tunes &amp; License</h1>

    <p>
      Everything here — the liveries, the tunes, the write-ups — is work I've put together over about five
      years across Forza Horizon 5 and 6. If you find something useful, grab the share code, try the tune,
      reference it in your own build. That's what it's here for.
    </p>
    <p>
      If you're pulling it into a collection, a guide, or anything you're publishing elsewhere, credit
      <strong>JasonHunt3r @ The Livery</strong> and link back. That's the one ask.
    </p>

    <p class="license-line">
      This site's original content — liveries, tunes, and write-ups — is licensed under
      <a href="https://creativecommons.org/licenses/by-nc-nd/4.0/" target="_blank" rel="noopener license">Creative Commons BY-NC-ND 4.0</a>.
    </p>

    <h2>In plain terms</h2>
    <ul>
      <li>You can share and reference anything here.</li>
      <li>Credit JasonHunt3r @ The Livery (and link back) when you do.</li>
      <li>Don't publish modified versions — no tweaked copies of a tune or livery passed off as a variant.</li>
      <li>Don't use it commercially — no reselling tunes, no monetized collections built on this data — without asking first.</li>
    </ul>

    <h2>What this doesn't cover</h2>
    <p>
      The cars themselves, in-game assets, and manufacturer likenesses belong to Playground Games, Microsoft,
      and the respective car manufacturers. This site is fan-made content built on top of Forza Horizon, not a
      claim of ownership over the game or the cars in it. Car reference data is sourced from
      <a href="https://forza.fandom.com" target="_blank" rel="noopener">Forza Wiki</a> and
      <a href="https://www.forzamotorsport.net" target="_blank" rel="noopener">Forza Motorsport</a>, credited
      under their own terms.
    </p>

    <p class="license-back"><a href="/">← Back to the catalog</a></p>
  </article>
</template>

<style scoped>
.license-page h2 {
  font-family: 'Oswald', sans-serif;
  font-weight: 600;
  font-size: 18px;
  letter-spacing: .01em;
  text-transform: uppercase;
  margin: 28px 0 10px;
  color: var(--fg);
}
.license-page p,
.license-page li {
  max-width: 640px;
}
.license-page p + p {
  margin-top: 12px;
}
.license-page ul {
  margin: 0;
  padding-left: 20px;
  color: var(--muted);
  font-size: 15px;
  line-height: 1.6;
}
.license-page a {
  color: var(--accent);
  text-underline-offset: 2px;
}
.license-line {
  margin-top: 20px;
  color: var(--fg);
  font-weight: 600;
}
.license-back {
  margin-top: 28px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
}
</style>
