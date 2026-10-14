<template>
  <div
    class="td-complex"
    :data-has-data="data !== undefined"
  >
    <h4 class="td-complex__title">
      {{ title }}
    </h4>
    <slot />
  </div>
</template>

<script setup lang="ts">
const {
  title,
  data = undefined,
} = defineProps<{
  /** Component heading */
  title: string;
  /** Arbitrary data bag - not a supported typedown type */
  data?: Record<string, unknown>;
}>();
</script>
