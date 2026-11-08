<script setup lang="ts">
import {
  Sun, Moon, Loader,
} from '@lucide/vue';
import {
  useTheme,
} from '../composables/client-preferences/useTheme';
import TdButton from './TdButton.vue';
import ClientOnly from './utils/ClientOnly.vue';

const {
  isDark, toggle,
} = useTheme();
</script>

<template>
  <ClientOnly>
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

    <template #fallback>
      <!-- A hidden icon to avoid layout shift -->
      <TdButton
        label=""
        aria-hidden="true"
      >
        <Loader
          class="animate-spin"
          :size="18"
        />
      </TdButton>
    </template>
  </ClientOnly>
</template>
