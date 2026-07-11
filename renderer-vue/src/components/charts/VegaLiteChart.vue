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
import ChartInteractionBar from "./ChartInteractionBar.vue";
import ChartTooltip from "./ChartTooltip.vue";

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
const { error, interactionActive, tooltip, resetInteraction, zoomBy } =
  useVegaLiteView(host, hydratedSpec);
const showHoverGuide = computed(
  () =>
    tooltip.value.visible &&
    ["line", "trail", "regression", "area", "errorband"].includes(
      props.chartType,
    ),
);
const hoverGuideStyle = computed(() => ({
  left: `${tooltip.value.anchorX}px`,
  top: `${host.value?.offsetTop ?? 0}px`,
  height: `${host.value?.clientHeight ?? 0}px`,
}));
</script>

<template>
  <div
    class="vega-chart-shell"
    :data-interaction="interaction?.mode"
    @keydown.esc="resetInteraction()"
  >
    <div class="chart-toolbar-lane">
      <ChartInteractionBar
        v-if="interaction"
        :interaction="interaction"
        :active="interactionActive"
        @reset="resetInteraction()"
        @zoom-in="zoomBy(0.8)"
        @zoom-out="zoomBy(1.25)"
      />
    </div>
    <div
      ref="host"
      class="vega-chart"
      role="group"
      tabindex="0"
      :aria-label="chartLabel"
    />
    <div
      v-if="showHoverGuide"
      class="chart-hover-guide"
      :style="hoverGuideStyle"
      aria-hidden="true"
    />
    <ChartTooltip :state="tooltip" />
    <p v-if="error" class="empty chart-render-error" role="alert">
      Unable to render this chart: {{ error }}
    </p>
  </div>
</template>
