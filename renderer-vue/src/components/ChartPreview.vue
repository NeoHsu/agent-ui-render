<template>
  <div v-if="chartKind === 'line' && lineSeries.length" class="chart">
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
        v-for="item in lineSeries"
        :key="item.key"
        fill="none"
        :stroke="item.color"
        stroke-width="3"
        stroke-linecap="round"
        stroke-linejoin="round"
        :points="item.points"
      />
      <text
        v-for="(item, index) in lineSeries"
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

  <div v-else-if="scatterPoints.length" class="chart">
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
      <text x="54" y="18" :fill="chartColors[0]" class="svg-label">
        {{ scatterLabel }}
      </text>
      <circle
        v-for="item in scatterPoints"
        :key="item.key"
        class="scatter-point"
        :cx="item.cx"
        :cy="item.cy"
        r="5"
        :fill="chartColors[0]"
        opacity="0.88"
      >
        <title>{{ item.title }}</title>
      </circle>
    </svg>
  </div>

  <div v-else-if="pieSlices.length" class="pie-chart">
    <svg class="pie-svg" viewBox="0 0 320 320" role="img" :aria-label="chartLabel">
      <circle
        cx="160"
        cy="160"
        :r="pieRadius"
        fill="none"
        stroke="var(--agent-border-soft)"
        stroke-width="46"
      />
      <g transform="rotate(-90 160 160)">
        <circle
          v-for="item in pieSlices"
          :key="item.key"
          cx="160"
          cy="160"
          :r="pieRadius"
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
        {{ pieTotalLabel }}
      </text>
    </svg>
    <div class="pie-legend" aria-label="Composition legend">
      <div v-for="item in pieSlices" :key="`legend-${item.key}`" class="pie-legend-row">
        <span class="pie-marker" :style="{ background: item.color }" />
        <span class="pie-label">{{ item.label }}</span>
        <span class="pie-value">{{ item.valueText }}</span>
        <span class="pie-percent">{{ item.percentText }}</span>
      </div>
    </div>
  </div>

  <div v-else-if="barItems.length" class="bar-chart">
    <div v-for="(item, index) in barItems" :key="index" class="bar-row">
      <div class="bar-label">{{ item.label }}</div>
      <div class="bar-track">
        <div class="bar-fill" :style="{ width: item.width }" />
      </div>
      <div class="bar-value">{{ item.value }}</div>
    </div>
  </div>

  <DataTableBlock v-else :dataset="dataset" />
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Dataset, ViewIntent } from "../types";
import {
  barChartModel,
  chartAriaLabel,
  lineChartModel,
  pieChartModel,
  pieRadius,
  scatterChartModel,
  seriesColors,
} from "../chart-model";
import DataTableBlock from "./DataTableBlock.vue";
import { chartKindForView } from "../chart-selection";

const props = defineProps<{
  dataset: Dataset;
  view: ViewIntent;
}>();

const chartColors = seriesColors;
const chartLabel = computed(() => chartAriaLabel(props.view));
const chartKind = computed(() => chartKindForView(props.view, props.dataset));

const lineSeries = computed(() =>
  chartKind.value === "line" ? lineChartModel(props.dataset, props.view) : [],
);

const scatterModel = computed(() =>
  chartKind.value === "scatter"
    ? scatterChartModel(props.dataset, props.view)
    : { label: "", points: [] },
);
const scatterPoints = computed(() => scatterModel.value.points);
const scatterLabel = computed(() => scatterModel.value.label);

const pieData = computed(() =>
  chartKind.value === "pie"
    ? pieChartModel(props.dataset, props.view)
    : { slices: [], totalLabel: "—" },
);
const pieSlices = computed(() => pieData.value.slices);
const pieTotalLabel = computed(() => pieData.value.totalLabel);

const barItems = computed(() =>
  chartKind.value === "bar" ? barChartModel(props.dataset, props.view) : [],
);
</script>
