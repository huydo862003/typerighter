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
    :label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
    @click="toggle"
  >
    <Sun
      v-if="isDark"
      :size="18"
    />
    <Moon
      v-else
      :size="18"
    />
  </TdButton>
</template>
