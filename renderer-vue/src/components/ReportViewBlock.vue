<script setup lang="ts">
import { computed } from "vue";
import type { Dataset, ViewIntent } from "../types";
import { projectDatasetForView, safeClass, viewTitle } from "../format";
import ChartPreview from "./ChartPreview.vue";
import DataTableBlock from "./DataTableBlock.vue";

const props = defineProps<{
  view: ViewIntent;
  dataset?: Dataset;
  index: number;
}>();

const resolvedDataset = computed(() => props.dataset ?? null);
const title = computed(() => props.view.title || viewTitle(props.view, props.index));
const sectionClasses = computed(() => [
  "card",
  "view-card",
  `view-card-${safeClass(props.view.intent)}`,
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
    <template v-if="resolvedDataset">
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
