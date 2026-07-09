<script setup lang="ts">
import { computed } from "vue";
import type { Metric } from "../types";
import { formatMetric } from "../format";

const props = defineProps<{ metrics: Metric[] }>();

const metricItems = computed(() =>
  props.metrics.map((metric, index) => ({
    key: `metric-${index}`,
    label: metric.label,
    value: formatMetric(metric),
    deltaLabel: metric.delta?.label,
  })),
);
</script>

<template>
  <section v-if="metricItems.length" class="metrics" aria-label="Metrics">
    <article
      v-for="metric in metricItems"
      :key="metric.key"
      class="metric-card"
    >
      <div class="metric-label">{{ metric.label }}</div>
      <div class="metric-value">{{ metric.value }}</div>
      <div v-if="metric.deltaLabel" class="metric-delta">
        {{ metric.deltaLabel }}
      </div>
    </article>
  </section>
</template>
