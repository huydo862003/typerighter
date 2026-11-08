<script setup lang="ts">
import {
  Sun, Moon,
} from '@lucide/vue';
import {
  useTheme,
} from '../composables/client-preferences/useTheme';
import TdButton from './TdButton.vue';

const {
  isDark, toggle, lightToggleStyle, darkToggleStyle,
} = useTheme();
</script>

<template>
  <TdButton
    :label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
    @click="toggle"
  >
    <Sun
      :style="darkToggleStyle"
      :size="18"
    />
    <Moon
      :style="lightToggleStyle"
      :size="18"
    />
  </TdButton>
</template>
