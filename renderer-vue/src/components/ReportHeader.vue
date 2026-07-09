<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    summary?: string;
    eyebrow?: string;
  }>(),
  {
    eyebrow: "Structured report",
  },
);
</script>

<template>
  <header class="report-header">
    <p class="eyebrow">{{ eyebrow }}</p>
    <h1>{{ title }}</h1>
    <p v-if="summary" class="summary">{{ summary }}</p>
  </header>
</template>
