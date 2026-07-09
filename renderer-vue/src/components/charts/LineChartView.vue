<script setup lang="ts">
import type { LineSeries } from "../../chart-model";

defineProps<{
  series: LineSeries[];
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
      <polyline
        v-for="item in series"
        :key="item.key"
        fill="none"
        :stroke="item.color"
        stroke-width="3"
        stroke-linecap="round"
        stroke-linejoin="round"
        :points="item.points"
      />
      <text
        v-for="(item, index) in series"
        :key="`legend-${item.key}`"
        :x="54 + index * 170"
        y="18"
        :fill="item.color"
        class="svg-label"
      >
        {{ item.label }}
      </text>
    </svg>
  </div>
</template>
