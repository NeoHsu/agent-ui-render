<script setup lang="ts">
withDefaults(
  defineProps<{
    assumptions: string[];
    title?: string;
  }>(),
  {
    title: "Assumptions and limitations",
  },
);
</script>

<template>
  <section v-if="assumptions.length" class="card muted">
    <h2>{{ title }}</h2>
    <ul>
      <li
        v-for="(assumption, index) in assumptions"
        :key="`assumption-${index}`"
      >
        {{ assumption }}
      </li>
    </ul>
  </section>
</template>
