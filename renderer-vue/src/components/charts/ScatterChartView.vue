<script setup lang="ts">
import type { ScatterPoint } from "../../chart-model";

defineProps<{
  points: ScatterPoint[];
  label: string;
  color: string;
  chartLabel: string;
}>();
</script>

<template>
  <div class="chart">
    <svg viewBox="0 0 760 280" role="img" :aria-label="chartLabel">
      <rect
        x="54"
        y="22"
        width="682"
        height="204"
        rx="14"
        fill="var(--agent-chart-bg)"
        stroke="var(--agent-chart-border)"
      />
      <line x1="54" y1="226" x2="736" y2="226" stroke="var(--agent-chart-axis)" />
      <line x1="54" y1="22" x2="54" y2="226" stroke="var(--agent-chart-axis)" />
      <text x="54" y="18" :fill="color" class="svg-label">
        {{ label }}
      </text>
      <circle
        v-for="item in points"
        :key="item.key"
        class="scatter-point"
        :cx="item.cx"
        :cy="item.cy"
        r="5"
        :fill="color"
        opacity="0.88"
      >
        <title>{{ item.title }}</title>
      </circle>
    </svg>
  </div>
</template>
