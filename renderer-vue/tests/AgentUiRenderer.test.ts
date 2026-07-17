import { afterEach, describe, expect, it } from "vitest";
import { createApp, h, type App } from "vue";
import AgentUiRenderer from "../src/AgentUiRenderer.vue";
import type { Report } from "../src/types.js";

const footerText =
	"Structured report generated from validated input; payload text was escaped.";
const activeApps: App[] = [];

const report: Report = {
	schema: "ui.input.normalized",
	version: 1,
	title: "Quarterly Review",
	summary: "Validated report summary.",
	theme: "executive-clean",
	density: "compact",
	emphasis: "subtle",
	alerts: [
		{ level: "warning", title: "Data caveat", content: "Partial period." },
	],
	metrics: [
		{ label: "Revenue", value: 125_000, format: "currency", unit: "USD" },
	],
	insights: ["Revenue increased."],
	markdown: [{ title: "Narrative", content: "Source-faithful **analysis**." }],
	datasets: {
		results: {
			columns: [
				{ key: "name", label: "Name", type: "string" },
				{ key: "amount", label: "Amount", type: "number" },
				{ key: "status", label: "Status", type: "string" },
			],
			rows: [["Alpha", 125_000, "confirmed"]],
		},
	},
	views: [
		{
			intent: "precise_records",
			data: "results",
			columns: ["name", "status"],
			title: "Selected records",
		},
	],
	assumptions: ["The period is incomplete."],
};

function renderReport(input: Report): HTMLElement {
	const host = document.createElement("div");
	document.body.append(host);
	const app = createApp({
		render: () => h(AgentUiRenderer, { input }),
	});
	activeApps.push(app);
	app.mount(host);
	const root = host.querySelector<HTMLElement>(".agent-ui-render");
	if (!root) throw new Error("AgentUiRenderer did not mount its report root");
	return root;
}

function textFor(root: HTMLElement, selector: string): string {
	const element = root.querySelector(selector);
	if (!element) throw new Error(`Missing rendered element ${selector}`);
	return element.textContent?.trim() ?? "";
}

function textList(root: HTMLElement, selector: string): string[] {
	return [...root.querySelectorAll(selector)].map(
		(element) => element.textContent?.trim() ?? "",
	);
}

function expectOrderedText(root: HTMLElement, values: string[]): void {
	const text = root.textContent ?? "";
	let previousIndex = -1;
	for (const value of values) {
		const index = text.indexOf(value);
		expect(
			index,
			`${value} should render after the previous block`,
		).toBeGreaterThan(previousIndex);
		previousIndex = index;
	}
}

afterEach(() => {
	for (const app of activeApps.splice(0)) app.unmount();
	document.body.replaceChildren();
});

describe("AgentUiRenderer", () => {
	it("renders governed blocks in order with projected records", () => {
		const root = renderReport(report);

		expect({
			theme: root.dataset.theme,
			density: root.dataset.density,
			emphasis: root.dataset.emphasis,
		}).toEqual({
			theme: "executive-clean",
			density: "compact",
			emphasis: "subtle",
		});
		expect(textList(root, "th")).toEqual(["Name", "Status"]);
		expectOrderedText(root, [
			"Quarterly Review",
			"Data caveat",
			"Revenue",
			"Revenue increased.",
			"Narrative",
			"Selected records",
			"The period is incomplete.",
			footerText,
		]);
	});

	it("omits absent optional blocks but keeps provenance", () => {
		const root = renderReport({ schema: "ui.input.normalized", version: 2 });

		expect(textFor(root, "h1")).toBe("Agent UI Report");
		expect(root.querySelector(".alerts")).toBeNull();
		expect(root.querySelector(".metrics")).toBeNull();
		expect(root.querySelector(".report-views")).toBeNull();
		expect(textFor(root, "footer")).toBe(footerText);
	});
});
