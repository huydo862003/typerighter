<script setup lang="ts">
import {
  Sun, Moon,
} from '@lucide/vue';
import {
  useTheme,
} from '../../app/composables/useTheme';
import TdButton from './TdButton.vue';

const {
  isDark, toggle,
} = useTheme();
</script>

<template>
  <TdButton
    class="w-9 h-9 text-td-neutral-fg-muted group hover:opacity-70"
    :label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
    @click="toggle"
  >
    <Sun
      v-if="isDark"
      :size="18"
      class="group-hover:text-td-primary-solid transition-colors"
    />
    <Moon
      v-else
      :size="18"
      class="group-hover:text-td-primary-solid transition-colors"
    />
  </TdButton>
</template>
