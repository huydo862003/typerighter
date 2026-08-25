<script setup lang="ts">
import {
  getIndexUrl,
} from '@/shared';
import {
  useSiteConfig,
} from '@/client/app';

const {
  withBase,
} = useSiteConfig();
</script>

<template>
  <div class="pt-16 pb-24">
    <p class="text-td-4xl font-extrabold leading-none tracking-td-tight text-td-fg-muted">
      404
    </p>
    <h1 class="text-td-xl font-extrabold leading-td-tight tracking-td-tight mt-4">
      Page Not Found
    </h1>
    <p class="text-td-fg-muted mt-2 text-td-base leading-td-relaxed">
      The page you are looking for does not exist.
    </p>
    <a
      class="inline-flex items-center mt-6 text-td-primary-solid text-td-sm font-extrabold tracking-td-tight hover:text-td-primary-solid-hover"
      :href="withBase(getIndexUrl('/'))"
    >Go Home</a>
  </div>
</template>
