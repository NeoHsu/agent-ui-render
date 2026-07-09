<script setup lang="ts">
withDefaults(
  defineProps<{
    text?: string;
  }>(),
  {
    text: "Structured report generated from validated input.",
  },
);
</script>

<template>
  <footer class="footer">
    {{ text }}
  </footer>
</template>
