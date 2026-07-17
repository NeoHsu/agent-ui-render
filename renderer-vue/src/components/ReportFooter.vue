<script setup lang="ts">
withDefaults(
  defineProps<{
    text?: string;
  }>(),
  {
    text: "Structured report generated from validated input; payload text was escaped.",
  },
);
</script>

<template>
  <footer v-if="text" class="footer">
    {{ text }}
  </footer>
</template>
