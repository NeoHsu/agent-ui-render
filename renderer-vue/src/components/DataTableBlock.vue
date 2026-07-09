<script setup lang="ts">
import { computed } from "vue";
import type { Dataset } from "../types";
import { cellValueClass, formatCell, tableCellClass } from "../format";

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
  <div class="table-wrap">
    <table>
      <caption>
        {{ caption }}
      </caption>
      <thead>
        <tr>
          <th v-for="column in dataset.columns" :key="column.key">
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
              :class="tableCellClass(row[columnIndex] ?? null, column)"
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
