<template>
  <section v-if="metrics.length" class="metrics" aria-label="Metrics">
    <article
      v-for="(metric, index) in metrics"
      :key="`metric-${index}`"
      class="metric-card"
    >
      <div class="metric-label">{{ metric.label }}</div>
      <div class="metric-value">{{ formatMetric(metric) }}</div>
      <div v-if="metric.delta?.label" class="metric-delta">
        {{ metric.delta.label }}
      </div>
    </article>
  </section>
</template>

<script setup lang="ts">
import type { Metric } from "../types";
import { formatMetric } from "../format";

defineProps<{ metrics: Metric[] }>();
</script>
