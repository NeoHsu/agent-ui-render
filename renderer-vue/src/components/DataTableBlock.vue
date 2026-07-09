<template>
  <div class="table-wrap">
    <table>
      <caption>
        Dataset table
      </caption>
      <thead>
        <tr>
          <th v-for="column in dataset.columns" :key="column.key">
            {{ column.label || column.key }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="dataset.rows.length === 0">
          <td :colspan="dataset.columns.length" class="empty">No rows</td>
        </tr>
        <tr v-for="(row, rowIndex) in dataset.rows" v-else :key="rowIndex">
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
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import type { Dataset } from "../types";
import { cellValueClass, formatCell, tableCellClass } from "../format";

defineProps<{ dataset: Dataset }>();
</script>
