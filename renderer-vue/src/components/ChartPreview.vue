<script setup lang="ts">
import { computed } from "vue";
import type { Dataset, ViewIntent } from "../types";
import {
  barChartModel,
  chartAriaLabel,
  lineChartModels,
  pieChartModel,
  pieRadius,
  scatterChartModel,
  seriesColors,
  verticalBarChartModel,
} from "../chart-model";
import { barOrientationForView, chartKindForView } from "../chart-selection";
import DataTableBlock from "./DataTableBlock.vue";
import BarChartView from "./charts/BarChartView.vue";
import LineChartView from "./charts/LineChartView.vue";
import PieChartView from "./charts/PieChartView.vue";
import ScatterChartView from "./charts/ScatterChartView.vue";
import VerticalBarChartView from "./charts/VerticalBarChartView.vue";

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
const barOrientation = computed(() =>
  chartKind.value === "bar"
    ? barOrientationForView(props.view, props.dataset)
    : "horizontal",
);

const lineModels = computed(() =>
  chartKind.value === "line" ? lineChartModels(props.dataset, props.view) : [],
);

const scatterModel = computed(() =>
  chartKind.value === "scatter"
    ? scatterChartModel(props.dataset, props.view)
    : {
        label: "",
        xLabel: "",
        yLabel: "",
        xTicks: [],
        yTicks: [],
        points: [],
      },
);

const pieData = computed(() =>
  chartKind.value === "pie"
    ? pieChartModel(props.dataset, props.view)
    : { slices: [], totalLabel: "—" },
);
const pieSlices = computed(() => pieData.value.slices);
const pieTotalLabel = computed(() => pieData.value.totalLabel);

const barModel = computed(() =>
  chartKind.value === "bar" && barOrientation.value === "horizontal"
    ? barChartModel(props.dataset, props.view)
    : {
        groups: [],
        legend: [],
        axisStart: "0",
        axisEnd: "",
        sharedScale: true,
      },
);

const verticalBarModel = computed(() =>
  chartKind.value === "bar" && barOrientation.value === "vertical"
    ? verticalBarChartModel(props.dataset, props.view)
    : { groups: [], legend: [], yTicks: [] },
);
</script>

<template>
  <div
    v-if="chartKind === 'line' && lineModels.length"
    class="line-chart-grid"
    :data-faceted="lineModels.length > 1"
  >
    <LineChartView
      v-for="model in lineModels"
      :key="model.key"
      :model="model"
      :chart-label="chartLabel"
    />
  </div>
  <ScatterChartView
    v-else-if="scatterModel.points.length"
    :model="scatterModel"
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
  <VerticalBarChartView
    v-else-if="verticalBarModel.groups.length"
    :model="verticalBarModel"
    :chart-label="chartLabel"
  />
  <BarChartView v-else-if="barModel.groups.length" :model="barModel" />
  <DataTableBlock v-else :dataset="dataset" :caption="fallbackCaption" />
</template>
