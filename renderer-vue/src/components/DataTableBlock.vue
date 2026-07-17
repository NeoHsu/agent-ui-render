<script setup lang="ts">
import { computed } from "vue";
import type { Dataset } from "../types";
import {
  cellValueClass,
  formatCell,
  safeClass,
  tableCellClass,
} from "../format";

const props = withDefaults(
  defineProps<{
    dataset: Dataset;
    caption?: string;
    emptyMessage?: string;
  }>(),
  {
    caption: "Dataset table",
    emptyMessage: "No rows",
  },
);

const columnCount = computed(() => Math.max(props.dataset.columns.length, 1));
</script>

<template>
  <div class="table-wrap" role="region" :aria-label="caption" tabindex="0">
    <table>
      <caption>
        {{ caption }}
      </caption>
      <thead>
        <tr>
          <th
            v-for="column in dataset.columns"
            :key="column.key"
            :class="`column-${safeClass(column.type ?? 'string')}`"
          >
            {{ column.label || column.key }}
          </th>
        </tr>
      </thead>
      <tbody>
        <template v-if="dataset.rows.length === 0">
          <tr>
            <td :colspan="columnCount" class="empty">{{ emptyMessage }}</td>
          </tr>
        </template>
        <template v-else>
          <tr v-for="(row, rowIndex) in dataset.rows" :key="rowIndex">
            <td
              v-for="(column, columnIndex) in dataset.columns"
              :key="column.key"
              :class="[
                tableCellClass(row[columnIndex] ?? null, column),
                `column-${safeClass(column.type ?? 'string')}`,
              ]"
            >
              <span :class="cellValueClass(row[columnIndex] ?? null, column)">
                {{ formatCell(row[columnIndex] ?? null, column) }}
              </span>
            </td>
          </tr>
        </template>
      </tbody>
    </table>
  </div>
</template>
