<script setup lang="ts">
import { ref } from 'vue'
import { useModalStore } from '../stores/modal'
import { useFactoidSchema } from '../composables/useFactoidSchema'

const modal = useModalStore()
const { schema, addType, removeType, renameType, addOption, removeOption, moveType } = useFactoidSchema()

// Inline rename state: key → draft value
const renameDrafts = ref<Record<string, string>>({})
const renameActive = ref<string | null>(null)

function startRename(key: string, current: string) {
  renameDrafts.value[key] = current
  renameActive.value = key
}
function confirmRename(key: string) {
  const name = renameDrafts.value[key]?.trim()
  if (name) renameType(key, name)
  renameActive.value = null
}
function cancelRename(key: string) {
  renameDrafts.value[key] = ''
  renameActive.value = null
}

// Add-option input per type key
const addInputs = ref<Record<string, string>>({})
const addInputRefs = ref<Record<string, HTMLInputElement | null>>({})

function confirmAddOption(key: string) {
  const v = addInputs.value[key]?.trim()
  if (!v) return
  addOption(key, v)
  addInputs.value[key] = ''
}

// Add type
const newTypeName = ref('')

function confirmAddType() {
  const name = newTypeName.value.trim()
  if (!name) return
  addType(name)
  newTypeName.value = ''
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="modal.factoidPanelOpen"
      class="fp-backdrop"
      @click.self="modal.closeFactoidPanel()"
    ></div>
    <div class="fp-panel" :class="{ open: modal.factoidPanelOpen }" :style="{ pointerEvents: modal.factoidPanelOpen ? 'auto' : 'none' }">
      <div class="fp-header">
        <span class="fp-title">Factoid Types</span>
        <button class="fp-close" type="button" @click="modal.closeFactoidPanel()">×</button>
      </div>

      <div class="fp-body">
        <div v-for="(type, idx) in schema" :key="type.key" class="fp-type">
          <div class="fp-type-head">
            <!-- Inline-editable name -->
            <div v-if="renameActive === type.key" class="fp-rename">
              <input
                class="fp-rename-input"
                v-model="renameDrafts[type.key]"
                @keydown.enter.prevent="confirmRename(type.key)"
                @keydown.escape="cancelRename(type.key)"
              />
              <button class="fp-btn-icon fp-btn-confirm" type="button" @click="confirmRename(type.key)">✓</button>
              <button class="fp-btn-icon" type="button" @click="cancelRename(type.key)">×</button>
            </div>
            <button
              v-else
              class="fp-type-name"
              type="button"
              @click="startRename(type.key, type.name)"
            >{{ type.name }}</button>

            <div class="fp-type-controls">
              <button
                class="fp-btn-icon fp-btn-move"
                type="button"
                :disabled="idx === 0"
                title="Move up"
                @click="moveType(type.key, -1)"
              >↑</button>
              <button
                class="fp-btn-icon fp-btn-move"
                type="button"
                :disabled="idx === schema.length - 1"
                title="Move down"
                @click="moveType(type.key, 1)"
              >↓</button>
              <button
                class="fp-btn-icon fp-btn-delete"
                type="button"
                title="Delete type"
                @click="removeType(type.key)"
              >×</button>
            </div>
          </div>

          <!-- Options pills -->
          <div class="fp-options">
            <span
              v-for="opt in type.options"
              :key="opt"
              class="fp-pill"
            >{{ opt }}<button class="fp-pill-x" type="button" @click="removeOption(type.key, opt)">×</button></span>

            <div class="fp-add-option">
              <input
                :ref="(el) => { addInputRefs[type.key] = el as HTMLInputElement | null }"
                class="fp-add-input"
                v-model="addInputs[type.key]"
                :placeholder="'Add ' + type.name + '…'"
                @keydown.enter.prevent="confirmAddOption(type.key)"
              />
              <button
                class="fp-btn-icon fp-btn-confirm"
                type="button"
                @click="confirmAddOption(type.key)"
              >＋</button>
            </div>
          </div>
        </div>

        <!-- Add new type -->
        <div class="fp-add-type">
          <input
            class="fp-add-input fp-add-type-input"
            v-model="newTypeName"
            placeholder="New factoid type…"
            @keydown.enter.prevent="confirmAddType"
          />
          <button
            class="fp-btn-icon fp-btn-confirm"
            type="button"
            @click="confirmAddType"
          >＋</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.fp-backdrop {
  position: fixed;
  inset: 0;
  z-index: 199;
  background: rgba(0, 0, 0, 0.35);
}

.fp-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: 320px;
  max-width: 90vw;
  z-index: 200;
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border-left: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  transform: translateX(100%);
  transition: transform 0.22s ease;
  overflow: hidden;
}

.fp-panel.open {
  transform: translateX(0);
}

.fp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 14px;
  border-bottom: 1px solid var(--panel-edge);
  flex-shrink: 0;
}

.fp-title {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .1em;
  text-transform: uppercase;
  color: var(--muted);
}

.fp-close {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 18px;
  line-height: 1;
  padding: 0 2px;
  cursor: pointer;
  opacity: 0.6;
  transition: opacity 0.12s, color 0.12s;
}
.fp-close:hover { opacity: 1; color: var(--fg); }

.fp-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.fp-type {
  border: 1px solid var(--panel-edge);
  border-radius: 5px;
  overflow: hidden;
}

.fp-type-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  background: var(--panel-well);
  border-bottom: 1px solid var(--panel-edge);
}

.fp-type-name {
  flex: 1;
  background: none;
  border: none;
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: .04em;
  text-align: left;
  cursor: pointer;
  padding: 0;
  opacity: 0.85;
  transition: opacity 0.12s, color 0.12s;
}
.fp-type-name:hover { opacity: 1; color: var(--accent); }

.fp-rename {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 4px;
}

.fp-rename-input {
  flex: 1;
  background: var(--panel);
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--fg);
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  padding: 2px 6px;
  outline: none;
}

.fp-type-controls {
  display: flex;
  align-items: center;
  gap: 2px;
}

.fp-btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 12px;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 3px;
  color: var(--muted);
  opacity: 0.6;
  transition: opacity 0.12s, color 0.12s, background 0.12s;
}
.fp-btn-icon:hover { opacity: 1; }
.fp-btn-icon:disabled { opacity: 0.2; cursor: default; }

.fp-btn-confirm { color: var(--accent); }
.fp-btn-confirm:hover { color: var(--accent-bright); }
.fp-btn-delete:hover { color: var(--danger-bright); opacity: 1; }
.fp-btn-move { font-size: 13px; }

.fp-options {
  padding: 10px 10px 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.fp-pill {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 100px;
  padding: 2px 8px 2px 10px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--muted);
}

.fp-pill-x {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 11px;
  line-height: 1;
  padding: 0 1px;
  cursor: pointer;
  opacity: 0.5;
  transition: color 0.1s, opacity 0.1s;
}
.fp-pill-x:hover { color: var(--danger-bright); opacity: 1; }

.fp-add-option {
  display: flex;
  align-items: center;
  gap: 3px;
  flex: 1;
  min-width: 120px;
}

.fp-add-input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  border-bottom: 1px solid var(--panel-edge);
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 2px 0;
  outline: none;
  transition: border-color 0.12s;
}
.fp-add-input:focus { border-color: var(--accent); }
.fp-add-input::placeholder { opacity: 0.4; }

.fp-add-type {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 12px;
  border: 1px dashed var(--panel-edge);
  border-radius: 5px;
}

.fp-add-type-input {
  font-size: 12px;
  color: var(--fg);
}
</style>
