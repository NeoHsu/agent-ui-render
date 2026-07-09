<script setup lang="ts">
import type { PieSlice } from "../../chart-model";

defineProps<{
  slices: PieSlice[];
  radius: number;
  totalLabel: string;
  chartLabel: string;
}>();
</script>

<template>
  <div class="pie-chart">
    <svg class="pie-svg" viewBox="0 0 320 320" role="img" :aria-label="chartLabel">
      <circle
        cx="160"
        cy="160"
        :r="radius"
        fill="none"
        stroke="var(--agent-border-soft)"
        stroke-width="46"
      />
      <g transform="rotate(-90 160 160)">
        <circle
          v-for="item in slices"
          :key="item.key"
          cx="160"
          cy="160"
          :r="radius"
          fill="none"
          :stroke="item.color"
          stroke-width="46"
          stroke-linecap="butt"
          :stroke-dasharray="item.dashArray"
          :stroke-dashoffset="item.dashOffset"
        />
      </g>
      <text x="160" y="154" text-anchor="middle" class="pie-total-label">Total</text>
      <text x="160" y="180" text-anchor="middle" class="pie-total-value">
        {{ totalLabel }}
      </text>
    </svg>
    <div class="pie-legend" aria-label="Composition legend">
      <div v-for="item in slices" :key="`legend-${item.key}`" class="pie-legend-row">
        <span class="pie-marker" :style="{ background: item.color }" />
        <span class="pie-label">{{ item.label }}</span>
        <span class="pie-value">{{ item.valueText }}</span>
        <span class="pie-percent">{{ item.percentText }}</span>
      </div>
    </div>
  </div>
</template>
