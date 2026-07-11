<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  shallowRef,
  useTemplateRef,
} from "vue";
import type { TopLevelSpec } from "vega-lite";
import {
  attachDatasets,
  chartInteraction,
  sizeVegaSpec,
} from "../../chart-data.js";
import { useVegaLiteView } from "../../composables/useVegaLiteView.js";
import type { Dataset, VegaLiteSpec } from "../../types.js";

const props = defineProps<{
  spec: VegaLiteSpec;
  datasetIds: string[];
  datasets: Record<string, Dataset>;
  chartLabel: string;
  chartType: string;
}>();

const host = useTemplateRef<HTMLElement>("host");
const chartWidth = shallowRef(520);
let resizeObserver: ResizeObserver | null = null;

function updateWidth(): void {
  const width = host.value?.clientWidth ?? 0;
  if (width > 0 && Math.abs(width - chartWidth.value) > 4) {
    chartWidth.value = width;
  }
}

onMounted(() => {
  updateWidth();
  if (!host.value) return;
  resizeObserver = new ResizeObserver(updateWidth);
  resizeObserver.observe(host.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});

const interaction = computed(() => chartInteraction(props.spec));
const hydratedSpec = computed(
  () =>
    attachDatasets(
      sizeVegaSpec(props.spec, props.chartType, chartWidth.value),
      props.datasetIds,
      props.datasets,
    ) as unknown as TopLevelSpec,
);
const { error, tooltip, resetInteraction } = useVegaLiteView(host, hydratedSpec);
const tooltipStyle = computed(() => ({
  left: `${tooltip.value.x}px`,
  top: `${tooltip.value.y}px`,
}));
</script>

<template>
  <div
    class="vega-chart-shell"
    :data-interaction="interaction?.mode"
    @keydown.esc="resetInteraction()"
  >
    <div v-if="interaction" class="chart-interaction-bar">
      <span class="chart-interaction-hint">{{ interaction.label }}</span>
      <button
        v-if="interaction.resettable"
        type="button"
        class="chart-reset-button"
        @click="resetInteraction()"
      >
        Reset
      </button>
    </div>
    <div
      ref="host"
      class="vega-chart"
      role="group"
      tabindex="0"
      :aria-label="chartLabel"
    />
    <div
      v-if="tooltip.visible"
      class="chart-tooltip"
      role="tooltip"
      :style="tooltipStyle"
    >
      <div
        v-for="(entry, index) in tooltip.entries"
        :key="`${entry.label}-${index}`"
        class="chart-tooltip-row"
      >
        <span class="chart-tooltip-label">{{ entry.label }}</span>
        <strong class="chart-tooltip-value">{{ entry.value }}</strong>
      </div>
    </div>
    <p v-if="error" class="empty chart-render-error" role="alert">
      Unable to render this chart: {{ error }}
    </p>
  </div>
</template>
