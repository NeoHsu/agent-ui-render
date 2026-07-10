<script setup lang="ts">
import type { BarChartModel } from "../../chart-model";

defineProps<{
  model: BarChartModel;
}>();
</script>

<template>
  <div class="bar-chart" :data-series-count="model.legend.length">
    <div v-if="model.legend.length > 1" class="chart-legend">
      <span
        v-for="item in model.legend"
        :key="item.key"
        class="chart-legend-item"
      >
        <span class="chart-legend-marker" :style="{ background: item.color }" />
        {{ item.label }}
      </span>
    </div>

    <div v-for="group in model.groups" :key="group.key" class="bar-group">
      <div class="bar-label">{{ group.label }}</div>
      <div class="bar-series-list">
        <div v-for="item in group.series" :key="item.key" class="bar-series-row">
          <div v-if="model.legend.length > 1" class="bar-series-name">
            {{ item.label }}
          </div>
          <div class="bar-track">
            <div
              class="bar-fill"
              :style="{ width: item.width, background: item.color }"
            >
              <span
                v-if="item.labelPlacement === 'inside'"
                class="bar-value-inside"
              >
                {{ item.value }}
              </span>
            </div>
          </div>
          <div v-if="item.labelPlacement === 'outside'" class="bar-value">
            {{ item.value }}
          </div>
          <div v-else class="bar-value bar-value-placeholder" aria-hidden="true" />
        </div>
      </div>
    </div>

    <div v-if="model.groups.length" class="bar-axis" :data-shared="model.sharedScale">
      <span>{{ model.axisStart }}</span>
      <span>{{ model.axisEnd }}</span>
    </div>
  </div>
</template>
