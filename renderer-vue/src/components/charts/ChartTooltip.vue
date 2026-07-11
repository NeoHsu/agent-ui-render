<script setup lang="ts">
import { computed } from "vue";
import type { VegaTooltipState } from "../../composables/useVegaLiteView.js";

const props = defineProps<{
  state: VegaTooltipState;
}>();

const position = computed(() => ({
  left: `${props.state.x}px`,
  top: `${props.state.y}px`,
}));
const swatchStyle = computed(() =>
  props.state.color ? { backgroundColor: props.state.color } : undefined,
);
</script>

<template>
  <div
    v-if="state.visible"
    class="chart-tooltip"
    role="tooltip"
    :style="position"
  >
    <div v-if="state.title || state.color" class="chart-tooltip-header">
      <span
        v-if="state.color"
        class="chart-tooltip-swatch"
        :style="swatchStyle"
        aria-hidden="true"
      />
      <strong v-if="state.title" class="chart-tooltip-title">
        {{ state.title }}
      </strong>
    </div>
    <div
      v-for="(entry, index) in state.entries"
      :key="`${entry.label}-${index}`"
      class="chart-tooltip-row"
    >
      <span class="chart-tooltip-label">{{ entry.label }}</span>
      <strong class="chart-tooltip-value">{{ entry.value }}</strong>
    </div>
  </div>
</template>
