<script setup lang="ts">
import {
  Content, useTdContent,
  useRoute,
} from 'typerighter/client';

const route = useRoute();

useTdContent(route);
</script>

<template>
  <div
    class="custom-layout"
    data-testid="custom-layout"
  >
    <header data-testid="custom-header">
      Custom Layout
    </header>
    <main>
      <Content />
    </main>
  </div>
</template>
