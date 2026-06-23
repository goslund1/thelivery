<script setup lang="ts">
import { reactive, computed, provide } from 'vue'
import type { Card } from '../types'
import { useUiStore } from '../stores/ui'
import { MarkDirtyKey } from '../keys'
import CardMeta from './CardMeta.vue'
import Gallery from './Gallery.vue'
import TagCloud from './TagCloud.vue'
import CollapsibleSection from './CollapsibleSection.vue'
import TextSection from './TextSection.vue'
import RecipeSection from './RecipeSection.vue'

const props = defineProps<{ card: Card }>()
const ui = useUiStore()

// Descendant editors mark this specific card dirty via inject.
provide(MarkDirtyKey, () => ui.markCardDirty(props.card.id))

// Per-section open state, so a parent action (Build It) can force one open and
// the collapsibles still follow the per-type expand filters.
const openState = reactive<Record<string, boolean>>({})
const recipeKey = computed(
  () => props.card.sections.find((s) => s.type === 'forza_recipe')?.key,
)
function onBuildIt() {
  if (recipeKey.value) openState[recipeKey.value] = true
}
</script>

<template>
  <div class="card" :class="{ 'legend-card': card.isLegend }" :data-collections="card.collections.join(',')">
    <CardMeta :card="card" />
    <Gallery :card="card" />
    <TagCloud :card="card" :recipe-key="recipeKey" @build-it="onBuildIt" />

    <!-- Generic, ordered, type-dispatched sections -->
    <CollapsibleSection
      v-for="section in card.sections"
      :key="section.key"
      :section-key="section.key"
      :label="section.label"
      :dom-id="`${section.key}-${card.id}`"
      v-model:open="openState[section.key]"
    >
      <TextSection v-if="section.type === 'text'" :card-id="card.id" :section="section" />
      <RecipeSection v-else-if="section.type === 'forza_recipe'" :recipe="section" />
    </CollapsibleSection>

  </div>
</template>

