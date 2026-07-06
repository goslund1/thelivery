<script setup lang="ts">
import { reactive, computed, provide, ref, watch } from 'vue'
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
  if (s.type === 'forza_recipe') return !s.variants?.length
    && !s.tuneName.trim() && !s.shareCode.trim()
    && s.upgrades.every(c => c.parts.length === 0) && s.adjustments.length === 0
  return false
}
const visibleSections = computed(() =>
  ui.isEditing ? props.card.sections : props.card.sections.filter(s => !isSectionEmpty(s))
)

// Bumped when the card object reference changes (e.g. history restore) to signal
// RecipeSection to re-read from props rather than treating it as its own round-trip.
const recipeResetToken = ref(0)
watch(() => props.card, (card) => {
  recipeResetToken.value++
  // Re-seed section open state from the restored card's defaultOpen values.
  for (const s of card.sections) {
    if (s.defaultOpen === false) openState[s.key] = false
    else delete openState[s.key]
  }
}, { deep: false })

// Per-section open state. Seeded from section.defaultOpen when set by the author
// (via EditCardModal/NewCardModal save); undefined means the global toggle controls it.
const openState = reactive<Record<string, boolean>>(
  Object.fromEntries(
    props.card.sections
      .filter(s => s.defaultOpen === false)
      .map(s => [s.key, false])
  )
)
const recipeKey = computed(
  () => props.card.sections.find((s) => s.type === 'forza_recipe')?.key,
)
function onBuildIt() {
  if (recipeKey.value) openState[recipeKey.value] = true
}

// Active car ID for gallery filtering. Set by RecipeSection when a variant tab is selected.
// Null = no filter (single-car cards or no tab selection yet).
const activeCarId = ref<string | null>(null)

// Ref to RecipeSection so we can call addVariantWithLookup when the interrupt fires.
type RecipeSectionInstance = InstanceType<typeof RecipeSection>
const recipeSectionRef = ref<RecipeSectionInstance | null>(null)

// Consume ui.pendingMultiCarTrigger if it targets this card.
watch(() => ui.pendingMultiCarTrigger, (trigger) => {
  if (!trigger || trigger.cardId !== props.card.id) return
  ui.consumeMultiCarTrigger()
  recipeSectionRef.value?.addVariantWithLookup(trigger.carId)
})
</script>

<template>
  <div class="card" :id="`card-${card.id}`" :class="{ 'legend-card': card.isLegend }" :data-collections="card.collections.join(',')" :style="card.accentOverride ? { '--accent': card.accentOverride } : undefined">
    <CardMeta :card="card" />
    <Gallery :card="card" :active-car-id="activeCarId" />
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
        ref="recipeSectionRef"
        :recipe="section"
        :card-id="card.id"
        :car-id="card.carId"
        :reset-token="recipeResetToken"
        @update:recipe="updated => Object.assign(section, updated)"
        @update:car-id="id => { cardsStore.setCarId(card.id, id); ui.markCardDirty(card.id) }"
        @update:active-car-id="id => { activeCarId = id }"
      />
    </CollapsibleSection>

  </div>
</template>

