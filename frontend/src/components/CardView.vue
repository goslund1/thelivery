<script setup lang="ts">
import { reactive, computed, provide } from 'vue'
import type { Card, Section } from '../types'
import { useUiStore } from '../stores/ui'
import { useCardsStore } from '../stores/cards'
import { MarkDirtyKey, CardIdKey } from '../keys'
import CardMeta from './CardMeta.vue'
import Gallery from './Gallery.vue'
import TagCloud from './TagCloud.vue'
import CollapsibleSection from './CollapsibleSection.vue'
import TextSection from './TextSection.vue'
import RecipeSection from './RecipeSection.vue'

const props = defineProps<{ card: Card }>()
const ui = useUiStore()
const cardsStore = useCardsStore()

provide(MarkDirtyKey, () => { ui.markCardDirty(props.card.id) })
provide(CardIdKey, props.card.id)

// Hide sections with no content in view mode — they're still present on the card
// and become visible in edit mode.
function isSectionEmpty(s: Section): boolean {
  if (s.type === 'text') return !s.body.trim() && !s.figurePath
  if (s.type === 'forza_recipe') return !s.tuneName.trim() && !s.shareCode.trim()
  return false
}
const visibleSections = computed(() =>
  ui.isEditing ? props.card.sections : props.card.sections.filter(s => !isSectionEmpty(s))
)

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
  <div class="card" :id="`card-${card.id}`" :class="{ 'legend-card': card.isLegend }" :data-collections="card.collections.join(',')">
    <CardMeta :card="card" />
    <Gallery :card="card" />
    <TagCloud :card="card" :recipe-key="recipeKey" @build-it="onBuildIt" />

    <!-- Generic, ordered, type-dispatched sections — filtered to non-empty in view mode -->
    <CollapsibleSection
      v-for="section in visibleSections"
      :key="section.key"
      :section-key="section.key"
      :label="section.label"
      :dom-id="`${section.key}-${card.id}`"
      v-model:open="openState[section.key]"
    >
      <TextSection v-if="section.type === 'text'" :card-id="card.id" :section="section" />
      <RecipeSection
        v-else-if="section.type === 'forza_recipe'"
        :recipe="section"
        :card-id="card.id"
        :car-id="card.carId"
        @update:recipe="updated => Object.assign(section, updated)"
        @update:car-id="id => { cardsStore.setCarId(card.id, id); ui.markCardDirty(card.id) }"
      />
    </CollapsibleSection>

  </div>
</template>

