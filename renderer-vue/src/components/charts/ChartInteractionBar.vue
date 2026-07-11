<script setup lang="ts">
import { computed } from "vue";
import type { ChartInteraction } from "../../chart-data.js";

const props = defineProps<{
  interaction: ChartInteraction;
  active: boolean;
}>();

defineEmits<{
  reset: [];
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
    <button
      v-if="interaction.resettable"
      type="button"
      class="chart-reset-button"
      aria-label="Clear chart interaction"
      :disabled="!active"
      @click="$emit('reset')"
    >
      <span aria-hidden="true">↺</span>
      <span>Clear</span>
    </button>
  </div>
</template>
