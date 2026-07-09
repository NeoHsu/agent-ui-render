<template>
  <section v-if="alerts.length" class="alerts" aria-label="Alerts">
    <article
      v-for="(alert, index) in alerts"
      :key="`alert-${index}`"
      class="alert"
      :class="`alert-${safeClass(alert.level)}`"
      :role="alert.level === 'error' || alert.level === 'critical' ? 'alert' : 'status'"
      :aria-label="alertLabel(alert)"
    >
      <strong v-if="alert.title">{{ alert.title }}</strong>
      <p>{{ alert.content }}</p>
    </article>
  </section>
</template>

<script setup lang="ts">
import type { Alert } from "../types";
import { safeClass } from "../format";

defineProps<{ alerts: Alert[] }>();

function alertLabel(alert: Alert): string {
  return `${alert.level} alert${alert.title ? `: ${alert.title}` : ""}`;
}
</script>
