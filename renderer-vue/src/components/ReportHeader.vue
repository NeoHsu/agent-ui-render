<script setup lang="ts">
defineProps<{
  title: string;
  summary?: string;
  eyebrow?: string;
}>();
</script>

<template>
  <header class="report-header">
    <p v-if="eyebrow" class="eyebrow">{{ eyebrow }}</p>
    <h1>{{ title }}</h1>
    <p v-if="summary" class="summary">{{ summary }}</p>
  </header>
</template>
