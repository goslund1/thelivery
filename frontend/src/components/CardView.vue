<script setup lang="ts">
import { reactive, computed, provide } from 'vue'
import type { Card, Section } from '../types'
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

// Card 0 is the instructions card — read-only, never editable.
const isReadOnly = computed(() => props.card.catalogNumber === 0)

// Descendant editors mark this specific card dirty via inject.
// No-op for read-only cards so edits don't register.
provide(MarkDirtyKey, () => { if (!isReadOnly.value) ui.markCardDirty(props.card.id) })

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
  <div class="card" :id="`card-${card.id}`" :class="{ 'legend-card': card.isLegend, 'read-only-card': isReadOnly }" :data-collections="card.collections.join(',')">
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
      <RecipeSection v-else-if="section.type === 'forza_recipe'" :recipe="section" />
    </CollapsibleSection>

  </div>
</template>

