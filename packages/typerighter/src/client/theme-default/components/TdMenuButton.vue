<script setup lang="ts">
import {
  Menu, X,
} from '@lucide/vue';
import {
  useMenu,
} from '../composables/useMenu';
import TdButton from './TdButton.vue';

const {
  isOpen, toggle,
} = useMenu();
</script>

<template>
  <TdButton
    class="td-menu-button"
    :label="isOpen ? 'Close menu' : 'Open menu'"
    :aria-expanded="isOpen"
    @click="toggle"
  >
    <X
      v-if="isOpen"
      :size="20"
    />
    <Menu
      v-else
      :size="20"
    />
  </TdButton>
</template>
