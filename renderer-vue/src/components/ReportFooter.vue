<script setup lang="ts">
defineProps<{
  text?: string;
}>();
</script>

<template>
  <footer v-if="text" class="footer">
    {{ text }}
  </footer>
</template>
