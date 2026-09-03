<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  useSiteConfig,
} from '../../app';

const config = useSiteConfig();
const year = new Date().getFullYear();

const hasFooter = computed(() => config.value.author !== undefined || config.value.license !== undefined);
</script>

<template>
  <footer
    v-if="hasFooter"
    class="td-footer"
  >
    <p
      v-if="config.license"
      class="td-footer-message"
    >
      Released under the {{ config.license }}.
    </p>
    <p
      v-if="config.author"
      class="td-footer-copyright"
    >
      Copyright &copy; {{ year }} {{ config.author }}
    </p>
  </footer>
</template>

<style scoped>
.td-footer {
  border-top: 1px solid var(--color-td-border);
  padding: 1.5rem 0;
  text-align: center;
  color: var(--color-td-gray-500);
  font-size: var(--font-size-td-sm);
}

.td-footer-message,
.td-footer-copyright {
  margin: 0;
  line-height: var(--line-height-td-normal);
}
</style>
