<template>
  <div
    class="td-tabs"
    :data-active-tab="activeTab"
  >
    <div
      class="td-tabs__bar"
      role="tablist"
    >
      <button
        v-for="(label, index) in tabLabels"
        :key="index"
        type="button"
        class="td-tabs__tab"
        :class="{
          'td-tabs__tab--active': activeTab === index,
        }"
        :aria-selected="String(activeTab === index)"
        role="tab"
        @click="() => activeTab = index"
      >
        {{ label }}
      </button>
    </div>
    <div class="td-tabs__panel">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  computed, ref,
} from 'vue';

const {
  labels,
  active = undefined,
} = defineProps<{
  /** Comma-separated tab labels */
  labels: string;
  /** Zero-based initial active tab index */
  active?: number;
}>();

const activeTab = ref(active !== undefined ? Number(active) : 0);
const tabLabels = computed(() => labels.split(',').map((item) => item.trim()));
</script>
