<script setup lang="ts">
import type { ScatterChartModel } from "../../chart-model";

defineProps<{
  model: ScatterChartModel;
  color: string;
  chartLabel: string;
}>();
</script>

<template>
  <div class="chart scatter-chart">
    <svg viewBox="0 0 760 300" role="img" :aria-label="chartLabel">
      <rect
        x="72"
        y="32"
        width="644"
        height="190"
        rx="12"
        fill="var(--agent-chart-bg)"
        stroke="var(--agent-chart-border)"
      />

      <g v-for="tick in model.yTicks" :key="tick.key">
        <line
          x1="72"
          :y1="tick.position"
          x2="716"
          :y2="tick.position"
          class="chart-grid-line"
        />
        <text
          x="62"
          :y="Number(tick.position) + 4"
          text-anchor="end"
          class="chart-axis-label"
        >
          {{ tick.label }}
        </text>
      </g>

      <g v-for="tick in model.xTicks" :key="tick.key">
        <line
          :x1="tick.position"
          y1="32"
          :x2="tick.position"
          y2="222"
          class="chart-grid-line chart-grid-line-vertical"
        />
        <text
          :x="tick.position"
          y="246"
          text-anchor="middle"
          class="chart-axis-label"
        >
          {{ tick.label }}
        </text>
      </g>

      <text x="394" y="282" text-anchor="middle" class="chart-axis-title">
        {{ model.xLabel }}
      </text>
      <text
        x="18"
        y="127"
        text-anchor="middle"
        class="chart-axis-title"
        transform="rotate(-90 18 127)"
      >
        {{ model.yLabel }}
      </text>
      <text x="72" y="19" :fill="color" class="svg-label">
        {{ model.label }}
      </text>

      <g v-for="item in model.points" :key="item.key">
        <circle
          class="scatter-point"
          :cx="item.cx"
          :cy="item.cy"
          r="6"
          :fill="color"
          stroke="var(--agent-surface)"
          stroke-width="2"
        >
          <title>{{ item.title }}</title>
        </circle>
        <text
          v-if="item.label"
          :x="Number(item.cx) + 9"
          :y="Number(item.cy) - 9"
          class="scatter-label"
        >
          {{ item.label }}
        </text>
      </g>
    </svg>
  </div>
</template>
