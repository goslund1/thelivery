<script setup lang="ts">
import { ref, provide } from 'vue'
import type { Livery } from '../types'
import { useUiStore } from '../stores/ui'
import { MarkDirtyKey } from '../keys'
import CardMeta from './CardMeta.vue'
import Gallery from './Gallery.vue'
import TagCloud from './TagCloud.vue'
import CollapsibleSection from './CollapsibleSection.vue'
import RecipeSection from './RecipeSection.vue'
import EditableText from './EditableText.vue'

const props = defineProps<{ livery: Livery }>()
const ui = useUiStore()

// Descendant editors mark this specific card dirty via inject.
provide(MarkDirtyKey, () => ui.markCardDirty(props.livery.id))

const recipeOpen = ref(false)
function onBuildIt() {
  recipeOpen.value = true
}
</script>

<template>
  <div class="card" :class="{ 'legend-card': livery.isLegend }" :data-collections="livery.collections.join(',')">
    <CardMeta :livery="livery" />
    <Gallery :livery="livery" />
    <TagCloud :livery="livery" @build-it="onBuildIt" />

    <CollapsibleSection section-key="inspiration" label="Inspiration">
      <div class="section-body gutter-layout">
        <div class="gutter-figure" :class="{ 'has-image': livery.inspiration.figurePath }" :data-group="livery.id">
          <img
            v-if="livery.inspiration.figurePath"
            class="gutter-figure-img"
            :src="livery.inspiration.figurePath"
            @click="ui.openLightbox(livery.inspiration.figurePath!)"
          />
          <button class="change-image-btn" type="button" @click="ui.openImagePicker(livery.id, 'inspiration')">Change Image</button>
        </div>
        <EditableText tag="div" class="anecdote-text" v-model="livery.inspiration.body" />
      </div>
    </CollapsibleSection>

    <CollapsibleSection section-key="notes" label="Design Notes">
      <div class="section-body gutter-layout">
        <div class="gutter-figure" :class="{ 'has-image': livery.designNotes.figurePath }" :data-group="livery.id">
          <img
            v-if="livery.designNotes.figurePath"
            class="gutter-figure-img"
            :src="livery.designNotes.figurePath"
            @click="ui.openLightbox(livery.designNotes.figurePath!)"
          />
          <button class="change-image-btn" type="button" @click="ui.openImagePicker(livery.id, 'notes')">Change Image</button>
        </div>
        <EditableText tag="div" class="gutter-text" v-model="livery.designNotes.body" />
      </div>
    </CollapsibleSection>

    <CollapsibleSection
      section-key="recipe"
      label="Tune / Build Parts"
      :dom-id="`recipe-${livery.id}`"
      v-model:open="recipeOpen"
    >
      <RecipeSection :recipe="livery.recipe" />
    </CollapsibleSection>
  </div>
</template>
