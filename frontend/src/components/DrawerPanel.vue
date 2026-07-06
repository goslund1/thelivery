<script setup lang="ts">
import { vScrollContain } from '../directives/scrollContain'

const props = withDefaults(defineProps<{
  open: boolean
  width?: number
  tabWidth?: number
  background?: string
  side?: 'left' | 'right'
  flush?: boolean
}>(), {
  width: 272,
  tabWidth: 14,
  side: 'left',
  flush: false,
})

const emit = defineEmits<{ 'update:open': [value: boolean] }>()
</script>

<template>
  <div
    v-scroll-contain
    class="dp-pane"
    :class="{ 'dp-pane--open': open, 'dp-pane--right': side === 'right', 'dp-pane--flush': flush }"
    :style="{ '--dp-w': width + 'px', '--dp-tab': tabWidth + 'px', background }"
  >
    <button
      v-if="side === 'right'"
      class="dp-tab"
      :class="{ 'dp-tab--open': open }"
      type="button"
      :title="open ? 'Collapse' : 'Expand'"
      @click="emit('update:open', !open)"
    >
      <slot name="tab">‹</slot>
    </button>

    <div class="dp-wing">
      <div v-if="$slots.header" class="dp-header">
        <slot name="header" />
      </div>
      <div class="dp-body">
        <slot />
      </div>
    </div>

    <button
      v-if="side !== 'right'"
      class="dp-tab"
      :class="{ 'dp-tab--open': open }"
      type="button"
      :title="open ? 'Collapse' : 'Expand'"
      @click="emit('update:open', !open)"
    >
      <slot name="tab">‹</slot>
    </button>
  </div>
</template>

<style scoped>
.dp-pane {
  --dp-w: 272px;
  --dp-tab: 14px;

  display: flex;
  flex-direction: row;
  align-items: stretch;
  width: var(--dp-tab);
  overflow: hidden;
  transition: width 0.22s ease;
  flex-shrink: 0;
  align-self: stretch;
  margin-top: 4px;
  margin-bottom: 4px;
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-right: none;
  border-radius: 6px 0 0 0;
}
.dp-pane.dp-pane--open {
  width: calc(var(--dp-w) + var(--dp-tab));
}

/* Flush: no margin, no radius — for embedding inside a bounded container */
.dp-pane--flush {
  margin: 0;
  border-radius: 0;
}

/* Right-side variant: tab on left edge, wing opens rightward */
.dp-pane--right {
  border-right: 1px solid var(--glass-border);
  border-left: none;
  border-radius: 0 6px 0 0;
}
.dp-pane--right .dp-tab {
  border-left: none;
  border-right: 1px solid rgba(255, 255, 255, 0.06);
}
.dp-pane--right .dp-tab::after {
  left: 0;
  right: -1px;
}

.dp-wing {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
}

.dp-header {
  padding: 12px 14px 10px;
  border-bottom: 1px solid var(--panel-edge);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}

.dp-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  padding: 14px;
  display: flex;
  flex-direction: column;
}

.dp-tab {
  flex-shrink: 0;
  width: var(--dp-tab);
  align-self: stretch;
  background: transparent;
  border: none;
  border-left: 1px solid rgba(255, 255, 255, 0.06);
  color: var(--muted);
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 10px 0 0;
  transition: color 0.15s, transform 0.22s;
  position: relative;
}
.dp-tab:hover { color: var(--accent); }
.dp-tab.dp-tab--open { transform: scaleX(-1); }

/* Separator line that appears between header and tab once open */
.dp-tab::after {
  content: '';
  position: absolute;
  left: -1px;
  right: 0;
  top: 36px;
  height: 1px;
  background: var(--panel-edge);
  opacity: 0;
  transition: opacity 0s 0.22s;
}
.dp-pane--open .dp-tab::after {
  opacity: 1;
  transition: opacity 0s;
}
</style>
