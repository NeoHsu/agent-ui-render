<script setup lang="ts">
import type { LineChartModel } from "../../chart-model";

defineProps<{
  model: LineChartModel;
  chartLabel: string;
}>();
</script>

<template>
  <div class="chart line-chart">
    <h3 v-if="model.title" class="chart-subtitle">{{ model.title }}</h3>
    <svg viewBox="0 0 760 270" role="img" :aria-label="chartLabel">
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

      <g v-for="(item, index) in model.series" :key="item.key">
        <line
          :x1="72 + index * 170"
          y1="15"
          :x2="88 + index * 170"
          y2="15"
          :stroke="item.color"
          stroke-width="4"
          stroke-linecap="round"
        />
        <text
          :x="94 + index * 170"
          y="19"
          :fill="item.color"
          class="svg-label"
        >
          {{ item.label }}
        </text>
        <polyline
          fill="none"
          :stroke="item.color"
          stroke-width="3"
          stroke-linecap="round"
          stroke-linejoin="round"
          :points="item.points"
        />
        <g v-for="point in item.markers" :key="point.key">
          <circle
            :cx="point.cx"
            :cy="point.cy"
            r="4.5"
            :fill="item.color"
            stroke="var(--agent-surface)"
            stroke-width="2"
          >
            <title>{{ point.title }}</title>
          </circle>
          <text
            v-if="model.showPointLabels"
            :x="point.cx"
            :y="Number(point.cy) - 9"
            text-anchor="middle"
            class="chart-point-label"
          >
            {{ point.label }}
          </text>
        </g>
      </g>
    </svg>
  </div>
</template>
