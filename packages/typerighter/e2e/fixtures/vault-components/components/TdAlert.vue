<template>
  <div class="td-alert" :data-kind="kind">
    <slot />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  kind?: string
}>()
</script>
