<script setup lang="ts">
import { computed } from "vue";
import type { Dataset, ViewIntent } from "../types";
import { projectDatasetForView, safeClass, viewTitle } from "../format";
import ChartPreview from "./ChartPreview.vue";
import DataTableBlock from "./DataTableBlock.vue";
import VegaLiteChart from "./charts/VegaLiteChart.vue";

const props = defineProps<{
  view: ViewIntent;
  dataset?: Dataset;
  datasets: Record<string, Dataset>;
  index: number;
  layout: "full" | "half";
}>();

const resolvedDataset = computed(() => props.dataset ?? null);
const isVegaChart = computed(
  () => props.view.intent === "chart" && Boolean(props.view.spec),
);
const chartType = computed(() => props.view.chart ?? "chart");
const title = computed(
  () =>
    props.view.title ||
    (isVegaChart.value
      ? chartType.value
          .split(/[-_]/)
          .filter(Boolean)
          .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
          .join(" ")
      : viewTitle(props.view, resolvedDataset.value, props.index)),
);
const chartDatasetIds = computed(
  () => props.view.datasets?.length ? props.view.datasets : [props.view.data],
);
const chartSpec = computed(() => props.view.spec ?? {});
const sectionClasses = computed(() => [
  "card",
  "view-card",
  `view-card-${safeClass(props.view.intent)}`,
  `view-card-${props.layout}`,
]);
const tableDataset = computed(() =>
  resolvedDataset.value
    ? projectDatasetForView(resolvedDataset.value, props.view)
    : null,
);
const tableCaption = computed(() => `${title.value} dataset`);
</script>

<template>
  <section
    :class="sectionClasses"
    :data-view-intent="view.intent"
    :data-view-priority="view.priority"
  >
    <h2>{{ title }}</h2>
    <VegaLiteChart
      v-if="isVegaChart"
      :spec="chartSpec"
      :dataset-ids="chartDatasetIds"
      :datasets="datasets"
      :chart-label="`${title} chart`"
      :chart-type="chartType"
    />
    <template v-else-if="resolvedDataset">
      <DataTableBlock
        v-if="view.intent === 'precise_records' && tableDataset"
        :dataset="tableDataset"
        :caption="tableCaption"
      />
      <ChartPreview
        v-else
        :dataset="resolvedDataset"
        :view="view"
        :fallback-caption="tableCaption"
      />
    </template>
    <p v-else class="empty">No dataset available for this view.</p>
  </section>
</template>
