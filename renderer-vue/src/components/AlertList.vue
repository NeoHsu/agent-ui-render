<script setup lang="ts">
import { computed } from "vue";
import type { Alert } from "../types";
import { safeClass } from "../format";

const props = defineProps<{ alerts: Alert[] }>();

const alertItems = computed(() =>
  props.alerts.map((alert, index) => ({
    key: `alert-${index}`,
    className: `alert-${safeClass(alert.level)}`,
    role: alert.level === "error" || alert.level === "critical" ? "alert" : "status",
    ariaLabel: `${alert.level} alert${alert.title ? `: ${alert.title}` : ""}`,
    title: alert.title,
    content: alert.content,
  })),
);
</script>

<template>
  <section v-if="alertItems.length" class="alerts" aria-label="Alerts">
    <article
      v-for="alert in alertItems"
      :key="alert.key"
      class="alert"
      :class="alert.className"
      :role="alert.role"
      :aria-label="alert.ariaLabel"
    >
      <strong v-if="alert.title">{{ alert.title }}</strong>
      <p>{{ alert.content }}</p>
    </article>
  </section>
</template>
