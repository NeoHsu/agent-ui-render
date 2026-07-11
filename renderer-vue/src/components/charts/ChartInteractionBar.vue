<script setup lang="ts">
import { computed } from "vue";
import type { ChartInteraction } from "../../chart-data.js";

const props = defineProps<{
  interaction: ChartInteraction;
  active: boolean;
}>();

defineEmits<{
  reset: [];
  zoomIn: [];
  zoomOut: [];
}>();

const statusLabel = computed(() => {
  if (!props.active) return props.interaction.label;
  return props.interaction.mode === "zoom" ? "Zoomed view" : "Selection active";
});
</script>

<template>
  <div
    class="chart-interaction-bar"
    :data-mode="interaction.mode"
    :data-active="active"
  >
    <span class="chart-interaction-status">
      <span class="chart-interaction-indicator" aria-hidden="true" />
      <span class="chart-interaction-hint">{{ statusLabel }}</span>
    </span>
    <div
      v-if="interaction.mode === 'zoom'"
      class="chart-zoom-controls"
      role="group"
      aria-label="Chart zoom controls"
    >
      <button
        type="button"
        class="chart-icon-button"
        aria-label="Zoom out"
        title="Zoom out"
        @click="$emit('zoomOut')"
      >
        −
      </button>
      <button
        type="button"
        class="chart-icon-button"
        aria-label="Zoom in"
        title="Zoom in"
        @click="$emit('zoomIn')"
      >
        +
      </button>
    </div>
    <button
      v-if="interaction.resettable && active"
      type="button"
      class="chart-reset-button"
      aria-label="Clear chart interaction"
      @click="$emit('reset')"
    >
      <span aria-hidden="true">↺</span>
      <span>Clear</span>
    </button>
  </div>
</template>
