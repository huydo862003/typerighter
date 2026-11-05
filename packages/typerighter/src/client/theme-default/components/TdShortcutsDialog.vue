<script setup lang="ts">
import {
  nextTick, useTemplateRef, watch,
} from 'vue';
import {
  X,
} from '@lucide/vue';
import {
  TdKeyName,
} from '../utils/keys';
import TdKbdShortcut from './TdKbdShortcut.vue';

const {
  open,
} = defineProps<{
  /** Whether the dialog is visible */
  open: boolean;
}>();
const emit = defineEmits<{
  close: [];
}>();
const dialogRef = useTemplateRef<HTMLElement>('dialog');

function close () {
  emit('close');
}

watch(() => open, (value) => {
  if (value) nextTick(() => dialogRef.value?.focus());
});
</script>

<template>
  <Teleport to="body">
    <template v-if="open">
      <div
        class="td-shortcuts-scrim"
        @click="close"
      />
      <div
        ref="dialog"
        class="td-shortcuts-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Keyboard shortcuts"
        tabindex="-1"
        @keydown.esc.stop="close"
      >
        <div class="td-shortcuts-header">
          <h2 class="td-shortcuts-title">
            Keyboard shortcuts
          </h2>
          <button
            class="td-shortcuts-close"
            type="button"
            aria-label="Close"
            @click="close"
          >
            <X :size="16" />
          </button>
        </div>
        <table class="td-shortcuts-table">
          <tbody>
            <tr>
              <td>Search</td>
              <td>
                <TdKbdShortcut
                  :keys="[
                    TdKeyName.Meta,
                    TdKeyName.k,
                  ]"
                />
              </td>
            </tr>
            <tr>
              <td>Commands</td>
              <td>
                <TdKbdShortcut
                  :keys="[
                    TdKeyName.Meta,
                    TdKeyName.FSlash,
                  ]"
                />
              </td>
            </tr>
            <tr>
              <td>Next page</td>
              <td>
                <TdKbdShortcut
                  :keys="[
                    TdKeyName.Control,
                    TdKeyName.ArrowRight,
                  ]"
                />
              </td>
            </tr>
            <tr>
              <td>Previous page</td>
              <td>
                <TdKbdShortcut
                  :keys="[
                    TdKeyName.Control,
                    TdKeyName.ArrowLeft,
                  ]"
                />
              </td>
            </tr>
            <tr>
              <td>Toggle sidebar</td>
              <td>
                <TdKbdShortcut
                  :keys="[
                    TdKeyName.Meta,
                    TdKeyName.Backslash,
                  ]"
                />
              </td>
            </tr>
            <tr>
              <td>Close</td>
              <td>
                <TdKbdShortcut :keys="[TdKeyName.Escape]" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </Teleport>
</template>

<style scoped>
.td-shortcuts-scrim {
  position: fixed;
  inset: 0;
  z-index: var(--layer-td-modal);
  background: var(--color-td-scrim);
}

.td-shortcuts-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: min(720px, calc(100vw - 64px));
  height: calc(100vh - 120px);
  z-index: var(--layer-td-modal);
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.16);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  outline: none;
}

.td-shortcuts-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--color-td-line);
}

.td-shortcuts-title {
  font-size: var(--font-size-td-base);
  font-weight: 600;
  color: var(--color-td-ink);
  margin: 0;
}

.td-shortcuts-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-td-ink-4);
  border-radius: 4px;
  transition:
    color var(--duration-td-fast) var(--ease-td-out-quart),
    background var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-shortcuts-close:hover {
  color: var(--color-td-ink);
  background: var(--color-td-hover);
}

.td-shortcuts-table {
  width: 100%;
  border-collapse: collapse;
  padding: 8px 0;
  flex: 1;
  overflow-y: auto;
}

.td-shortcuts-table td {
  padding: 10px 20px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink-3);
}

.td-shortcuts-table td:last-child {
  text-align: right;
}
</style>
