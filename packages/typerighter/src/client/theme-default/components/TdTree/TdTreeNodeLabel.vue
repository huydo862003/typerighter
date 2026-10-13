<script setup lang="ts">
import {
  useLazyInlineMarkup,
} from '../../utils/renderInlineMarkup';

const {
  text,
} = defineProps<{
  /** Text to render, may contain inline code or math */
  text: string;
}>();

const rendered = useLazyInlineMarkup(() => text);
</script>

<template>
  <span v-html="rendered" />
</template>
