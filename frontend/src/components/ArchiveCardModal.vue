<script setup lang="ts">
import { useModalStore } from '../stores/modal'

const modal = useModalStore()

function confirm() {
  modal.confirmArchiveCard()
}

function cancel() {
  modal.cancelArchiveCard()
}
</script>

<template>
  <div v-if="modal.archiveCardPending" class="archive-backdrop" @click.self="cancel">
    <div class="archive-dialog">
      <div class="archive-title">Remove "{{ modal.archiveCardName }}" from gallery?</div>
      <p class="archive-body">
        This card will be hidden from the gallery but <strong>not permanently deleted</strong>.
        You can restore it any time from <strong>Admin Panel → Tools → Deleted Cards</strong>.
      </p>
      <div class="archive-actions">
        <button class="archive-btn archive-btn--cancel" @click="cancel">Cancel</button>
        <button class="archive-btn archive-btn--confirm" @click="confirm">Remove from Gallery</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.archive-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1200;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  overscroll-behavior: contain;
}

.archive-dialog {
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 8px;
  padding: 24px 28px;
  max-width: 380px;
  width: 90vw;
  box-shadow: 0 8px 40px rgba(0,0,0,0.5);
}

.archive-title {
  font-family: 'Oswald', sans-serif;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--fg);
  margin-bottom: 12px;
}

.archive-body {
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  line-height: 1.6;
  color: var(--muted);
  margin: 0 0 20px;
}

.archive-body strong {
  color: var(--accent);
  font-weight: 600;
}

.archive-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.archive-btn {
  padding: 8px 18px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.archive-btn--cancel {
  border: 1px solid var(--panel-edge);
  background: var(--panel-well);
  color: var(--muted);
}
.archive-btn--cancel:hover { color: var(--fg); border-color: var(--fg); }

.archive-btn--confirm {
  border: 2px solid #7a0000;
  background: #5c0000;
  color: #fff;
}
.archive-btn--confirm:hover {
  background: #cc0000;
  border-color: #ff4444;
  box-shadow: 0 0 14px rgba(200, 0, 0, 0.7);
}
</style>
