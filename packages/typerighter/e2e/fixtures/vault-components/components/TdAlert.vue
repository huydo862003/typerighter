<template>
  <div
    class="td-alert"
    :data-kind="kind"
  >
    <slot />
  </div>
</template>

<script setup lang="ts">
const {
  kind = undefined,
} = defineProps<{
  /** Alert severity */
  kind?: string;
}>();
</script>
