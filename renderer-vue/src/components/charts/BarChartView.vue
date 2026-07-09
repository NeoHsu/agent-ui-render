<script setup lang="ts">
import type { BarItem } from "../../chart-model";

defineProps<{
  items: BarItem[];
}>();
</script>

<template>
  <div class="bar-chart">
    <div v-for="(item, index) in items" :key="index" class="bar-row">
      <div class="bar-label">{{ item.label }}</div>
      <div class="bar-track">
        <div class="bar-fill" :style="{ width: item.width }" />
      </div>
      <div class="bar-value">{{ item.value }}</div>
    </div>
  </div>
</template>
