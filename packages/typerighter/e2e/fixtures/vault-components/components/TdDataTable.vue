<template>
  <figure
    class="td-data-table"
    :class="{
      'td-data-table--striped': striped,
    }"
    :data-striped="striped"
  >
    <figcaption
      v-if="caption"
      class="td-data-table__caption"
    >
      {{ caption }}
    </figcaption>
    <div class="td-data-table__body">
      <slot />
    </div>
  </figure>
</template>

<script setup lang="ts">
const {
  caption = undefined,
  striped = false,
} = defineProps<{
  /** Table caption text */
  caption?: string;
  /** Alternate row striping */
  striped?: boolean;
}>();
</script>
