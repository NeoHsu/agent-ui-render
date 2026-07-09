<script setup lang="ts">
withDefaults(
  defineProps<{
    insights: string[];
    title?: string;
  }>(),
  {
    title: "重點洞察",
  },
);
</script>

<template>
  <section v-if="insights.length" class="card">
    <h2>{{ title }}</h2>
    <ul class="insights">
      <li v-for="(insight, index) in insights" :key="`insight-${index}`">
        {{ insight }}
      </li>
    </ul>
  </section>
</template>
