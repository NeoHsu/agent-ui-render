import { createApp } from "vue";
import AgentUiRenderer from "./AgentUiRenderer.vue";
import type { Report } from "./types";
import "./agent-ui.css";

export function mount(root: Element, input: Report): void {
  createApp(AgentUiRenderer, { input }).mount(root);
}

export function readEmbeddedPayload(id = "agent-ui-payload"): Report {
  const payload = document.getElementById(id)?.textContent;
  if (!payload) throw new Error(`Missing embedded Agent UI payload #${id}`);
  try {
    return JSON.parse(payload) as Report;
  } catch (error) {
    throw new Error(
      `Invalid embedded Agent UI payload: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

export function autoMount(): void {
  const root = document.getElementById("agent-ui-root");
  if (!root) return;
  mount(root, readEmbeddedPayload());
}

declare global {
  interface Window {
    AgentUiRender?: {
      mount: typeof mount;
      readEmbeddedPayload: typeof readEmbeddedPayload;
      autoMount: typeof autoMount;
    };
  }
}

window.AgentUiRender = { mount, readEmbeddedPayload, autoMount };

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", autoMount, { once: true });
} else {
  autoMount();
}
