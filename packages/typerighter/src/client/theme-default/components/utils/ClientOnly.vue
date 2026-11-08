<script setup lang="ts">
import {
  onMounted, ref,
} from 'vue';

const isMounted = ref(false);

onMounted(() => {
  isMounted.value = true;
});
</script>

<template>
  <slot v-if="isMounted" />
  <slot
    v-else
    name="fallback"
  />
</template>
