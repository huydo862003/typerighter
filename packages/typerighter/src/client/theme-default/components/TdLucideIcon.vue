<script setup lang="ts">
import {
  getPageIcon,
} from '../composables/pageIcon';

const {
  name,
} = defineProps<{
  /** Lucide icon name */
  name: string;
}>();

const icon = getPageIcon(name);
</script>

<template>
  <component
    :is="icon"
    v-if="icon"
    :size="14"
    class="td-inline-icon"
  />
</template>

<style scoped>
.td-inline-icon {
  display: inline;
  vertical-align: -2px;
  margin-right: 2px;
}
</style>
