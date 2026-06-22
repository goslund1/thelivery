<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { ForzaRecipeSection } from '../types'
import { useUiStore } from '../stores/ui'
import EditableText from './EditableText.vue'

const props = defineProps<{ recipe: ForzaRecipeSection }>()
const ui = useUiStore()

const specKeys = computed(() => Object.keys(props.recipe.coreSpecs))
const partCount = computed(() =>
  props.recipe.upgrades.reduce((n, c) => n + c.parts.length, 0),
)

// The Upgrades sub-list follows its own filter checkbox + expand/collapse-all.
const kitOpen = ref(false)
watch(() => ui.upgradesExpanded, (v) => (kitOpen.value = v))
function onKitToggle(e: Event) {
  kitOpen.value = (e.target as HTMLDetailsElement).open
}
</script>

<template>
  <div class="section-body">
    <div class="tune-header">
      <EditableText tag="p" class="tune-name" v-model="recipe.tuneName" />
      <div class="plate">SHARE CODE: <EditableText tag="b" v-model="recipe.shareCode" /></div>
    </div>

    <table class="recipe-table">
      <tbody>
        <tr>
          <th v-for="k in specKeys" :key="k" class="editable">{{ k }}</th>
        </tr>
        <tr>
          <td v-for="k in specKeys" :key="k">
            <EditableText v-model="recipe.coreSpecs[k]" />
          </td>
        </tr>
      </tbody>
    </table>

    <details class="kit-toggle" :open="kitOpen" @toggle="onKitToggle">
      <summary title="Click to expand or collapse the full parts list">
        <span class="kit-label-group">
          <span class="section-label">Upgrades Installed</span> — {{ partCount }} parts
        </span>
        <span class="chev"></span>
      </summary>
      <div class="kit-body">
        <div v-for="(cat, ci) in recipe.upgrades" :key="ci" class="kit-cat">
          <EditableText class="kit-cat-label" v-model="cat.category" />
          <ul class="kit-list">
            <li v-for="(_, pi) in cat.parts" :key="pi" class="editable-li">
              <EditableText v-model="cat.parts[pi]" />
            </li>
          </ul>
        </div>
      </div>
    </details>

    <p class="kit-cat-label adj-label">Tune Adjustments</p>
    <div class="adjustments-box">
      <ul class="recipe-adjustments">
        <li v-for="(adj, ai) in recipe.adjustments" :key="ai">
          <EditableText tag="b" v-model="adj.name" /> —
          <EditableText tag="span" v-model="adj.description" />
        </li>
      </ul>
    </div>
  </div>
</template>
