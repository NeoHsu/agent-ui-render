<template>
  <section v-if="insights.length" class="card">
    <h2>重點洞察</h2>
    <ul class="insights">
      <li v-for="(insight, index) in insights" :key="`insight-${index}`">
        {{ insight }}
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
defineProps<{ insights: string[] }>();
</script>
