<script setup lang="ts">
import type { VerticalBarChartModel } from "../../chart-model";

defineProps<{
  model: VerticalBarChartModel;
  chartLabel: string;
}>();
</script>

<template>
  <div class="chart vertical-bar-chart">
    <div v-if="model.legend.length" class="chart-legend">
      <span
        v-for="item in model.legend"
        :key="item.key"
        class="chart-legend-item"
      >
        <span class="chart-legend-marker" :style="{ background: item.color }" />
        {{ item.label }}
      </span>
    </div>

    <svg viewBox="0 0 760 270" role="img" :aria-label="chartLabel">
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

      <line x1="72" y1="32" x2="72" y2="222" class="chart-axis-line" />
      <line x1="72" y1="222" x2="716" y2="222" class="chart-axis-line" />

      <g v-for="group in model.groups" :key="group.key">
        <g v-for="bar in group.bars" :key="bar.key">
          <rect
            :x="bar.x"
            :y="bar.y"
            :width="bar.width"
            :height="bar.height"
            rx="3"
            :fill="bar.color"
            class="vertical-bar"
          >
            <title>{{ group.label }} · {{ bar.label }}: {{ bar.value }}</title>
          </rect>
          <text
            :x="bar.labelX"
            :y="bar.labelY"
            text-anchor="middle"
            class="vertical-bar-value"
          >
            {{ bar.value }}
          </text>
        </g>
        <text
          :x="group.labelX"
          y="246"
          text-anchor="middle"
          class="chart-axis-label vertical-bar-category"
        >
          {{ group.label }}
        </text>
      </g>
    </svg>
  </div>
</template>
