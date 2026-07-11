<script setup lang="ts">
import { computed } from "vue";
import type {
  UIEmphasis,
  UIDensity,
  Report,
  UITheme,
  ViewIntent,
} from "./types";
import AlertList from "./components/AlertList.vue";
import AssumptionList from "./components/AssumptionList.vue";
import InsightList from "./components/InsightList.vue";
import MarkdownBlock from "./components/MarkdownBlock.vue";
import MetricGrid from "./components/MetricGrid.vue";
import ReportFooter from "./components/ReportFooter.vue";
import ReportHeader from "./components/ReportHeader.vue";
import ReportViewBlock from "./components/ReportViewBlock.vue";

const props = defineProps<{
  input: Report;
  theme?: UITheme;
  density?: UIDensity;
  emphasis?: UIEmphasis;
}>();

const title = computed(() => props.input.title || "Agent UI Report");
const summary = computed(() => props.input.summary);
const themeName = computed(
  () => props.theme ?? props.input.theme ?? "report-light",
);
const densityName = computed(
  () => props.density ?? props.input.density ?? "comfortable",
);
const emphasisName = computed(
  () => props.emphasis ?? props.input.emphasis ?? "strong",
);

const alerts = computed(() => props.input.alerts ?? []);
const metrics = computed(() => props.input.metrics ?? []);
const insights = computed(() => props.input.insights ?? []);
const markdownSections = computed(() => props.input.markdown ?? []);
const views = computed(() => props.input.views ?? []);
const datasets = computed(() => props.input.datasets ?? {});
const orderedViews = computed(() => [
  ...views.value.filter((view) => view.intent !== "precise_records"),
  ...views.value.filter((view) => view.intent === "precise_records"),
]);
const chartViews = computed(() =>
  orderedViews.value.filter((view) => view.intent !== "precise_records"),
);
const useSplitChartLayout = computed(
  () =>
    chartViews.value.length > 1 &&
    !chartViews.value.some((view) => view.intent === "trend"),
);
const fullWidthChartTypes = new Set([
  "gantt",
  "bullet",
  "parallel-coordinates",
  "candlestick",
  "errorband",
  "trail",
  "layer",
  "facet",
  "concat",
  "repeat",
]);

function layoutForView(view: ViewIntent): "full" | "half" {
  if (view.intent === "precise_records") return "full";
  if (view.intent === "chart" && fullWidthChartTypes.has(view.chart ?? "")) {
    return "full";
  }
  return useSplitChartLayout.value ? "half" : "full";
}

const assumptions = computed(() => props.input.assumptions ?? []);
</script>

<template>
  <article
    class="agent-ui-render"
    :data-theme="themeName"
    :data-density="densityName"
    :data-emphasis="emphasisName"
  >
    <ReportHeader :title="title" :summary="summary" />

    <AlertList :alerts="alerts" />
    <MetricGrid :metrics="metrics" />
    <InsightList :insights="insights" />
    <MarkdownBlock
      v-for="(section, index) in markdownSections"
      :key="`markdown-${index}`"
      :title="section.title"
      :content="section.content"
    />

    <div v-if="orderedViews.length" class="report-views">
      <ReportViewBlock
        v-for="(view, index) in orderedViews"
        :key="`${view.data}-${view.intent}-${index}`"
        :view="view"
        :dataset="datasets[view.data]"
        :datasets="datasets"
        :index="index"
        :layout="layoutForView(view)"
      />
    </div>

    <AssumptionList :assumptions="assumptions" />
    <ReportFooter />
  </article>
</template>

<style src="./agent-ui.css"></style>
