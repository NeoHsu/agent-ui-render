<template>
  <article
    class="agent-ui-render"
    :data-theme="themeName"
    :data-density="densityName"
    :data-emphasis="emphasisName"
  >
    <header class="report-header">
      <p class="eyebrow">Structured report</p>
      <h1>{{ input.title || "Agent UI Report" }}</h1>
      <p v-if="input.summary" class="summary">{{ input.summary }}</p>
    </header>

    <AlertList :alerts="input.alerts ?? []" />
    <MetricGrid :metrics="input.metrics ?? []" />
    <InsightList :insights="input.insights ?? []" />
    <MarkdownBlock
      v-for="(section, index) in input.markdown ?? []"
      :key="`markdown-${index}`"
      :title="section.title"
      :content="section.content"
    />

    <section
      v-for="(view, index) in input.views ?? []"
      :key="`view-${index}`"
      class="card"
    >
      <h2>{{ view.title || viewTitle(view, index) }}</h2>
      <template v-if="datasetForView(view)">
        <DataTableBlock
          v-if="view.intent === 'precise_records'"
          :dataset="datasetForView(view)!"
        />
        <ChartPreview v-else :dataset="datasetForView(view)!" :view="view" />
      </template>
      <p v-else class="empty">No dataset available for this view.</p>
    </section>

    <section v-if="input.assumptions?.length" class="card muted">
      <h2>假設與限制</h2>
      <ul>
        <li
          v-for="(assumption, index) in input.assumptions"
          :key="`assumption-${index}`"
        >
          {{ assumption }}
        </li>
      </ul>
    </section>

    <footer class="footer">
      Structured report generated from validated input.
    </footer>
  </article>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type {
  UIEmphasis,
  UIDensity,
  Report,
  UITheme,
  ViewIntent,
} from "./types";
import { viewTitle } from "./format";
import AlertList from "./components/AlertList.vue";
import ChartPreview from "./components/ChartPreview.vue";
import DataTableBlock from "./components/DataTableBlock.vue";
import InsightList from "./components/InsightList.vue";
import MarkdownBlock from "./components/MarkdownBlock.vue";
import MetricGrid from "./components/MetricGrid.vue";

const props = defineProps<{
  input: Report;
  theme?: UITheme;
  density?: UIDensity;
  emphasis?: UIEmphasis;
}>();

const themeName = computed(
  () => props.theme ?? props.input.theme ?? "report-light",
);
const densityName = computed(
  () => props.density ?? props.input.density ?? "comfortable",
);
const emphasisName = computed(
  () => props.emphasis ?? props.input.emphasis ?? "strong",
);

function datasetForView(view: ViewIntent) {
  return props.input.datasets?.[view.data];
}
</script>

<style src="./agent-ui.css"></style>
