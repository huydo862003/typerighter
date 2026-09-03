<script setup lang="ts">
import {
  useSiteConfig,
} from '../../app';

const config = useSiteConfig();
const year = new Date().getFullYear();
</script>

<template>
  <footer class="border-t border-td-neutral-border bg-td-neutral-bg">
    <div class="mx-auto max-w-[1180px] flex items-center justify-between gap-5 flex-wrap px-7 py-6 font-mono text-[length:var(--font-size-td-2xs)] text-td-neutral-fg-muted">
      <span>
        Powered by
        <a
          href="https://github.com/huydo862003/typerighter"
          target="_blank"
          rel="noopener noreferrer"
          class="font-medium text-td-primary-solid hover:underline"
        >Typerighter</a>
      </span>
      <span v-if="config.author || config.license">
        <template v-if="config.license">{{ config.license }}</template>
        <template v-if="config.license && config.author"> · </template>
        <template v-if="config.author">&copy; {{ year }} {{ config.author }}</template>
      </span>
    </div>
  </footer>
</template>
