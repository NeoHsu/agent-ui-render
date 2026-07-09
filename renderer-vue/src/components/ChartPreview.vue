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
import { chartKindForView } from "../chart-selection";
import DataTableBlock from "./DataTableBlock.vue";
import BarChartView from "./charts/BarChartView.vue";
import LineChartView from "./charts/LineChartView.vue";
import PieChartView from "./charts/PieChartView.vue";
import ScatterChartView from "./charts/ScatterChartView.vue";

const props = withDefaults(
  defineProps<{
    dataset: Dataset;
    view: ViewIntent;
    fallbackCaption?: string;
  }>(),
  {
    fallbackCaption: "Dataset table",
  },
);

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

<template>
  <LineChartView
    v-if="chartKind === 'line' && lineSeries.length"
    :series="lineSeries"
    :chart-label="chartLabel"
  />
  <ScatterChartView
    v-else-if="scatterPoints.length"
    :points="scatterPoints"
    :label="scatterLabel"
    :color="chartColors[0]"
    :chart-label="chartLabel"
  />
  <PieChartView
    v-else-if="pieSlices.length"
    :slices="pieSlices"
    :radius="pieRadius"
    :total-label="pieTotalLabel"
    :chart-label="chartLabel"
  />
  <BarChartView v-else-if="barItems.length" :items="barItems" />
  <DataTableBlock v-else :dataset="dataset" :caption="fallbackCaption" />
</template>
