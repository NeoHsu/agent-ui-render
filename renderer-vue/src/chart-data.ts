import type { Dataset, VegaLiteSpec } from "./types.js";

export type VegaDatum = Record<string, string | number | boolean | null>;

export function datasetRows(dataset: Dataset): VegaDatum[] {
	const keys = dataset.columns.map((column) => column.key);
	return dataset.rows.map(
		(row) =>
			Object.fromEntries(
				keys.map((key, index) => [key, row[index] ?? null]),
			) as VegaDatum,
	);
}

function sizeUnitSpec(spec: VegaLiteSpec, width: number, height: number): void {
	spec.width = Math.max(220, Math.floor(width));
	spec.height = Math.max(220, Math.floor(height));
	spec.autosize = {
		type: "fit",
		contains: "padding",
		resize: true,
	};
}

function repeatCount(repeat: unknown): number {
	if (Array.isArray(repeat)) return repeat.length;
	if (!repeat || typeof repeat !== "object") return 1;
	const definition = repeat as Record<string, unknown>;
	const repeated = definition.column ?? definition.row;
	return Array.isArray(repeated) ? repeated.length : 1;
}

function sizeConcatSpec(spec: VegaLiteSpec, width: number): void {
	const horizontal = Array.isArray(spec.hconcat) ? spec.hconcat : null;
	const vertical = Array.isArray(spec.vconcat) ? spec.vconcat : null;
	const children = horizontal ?? vertical ?? [];
	const childWidth = horizontal
		? Math.max(
				240,
				(width - 28 * Math.max(children.length - 1, 0)) /
					Math.max(children.length, 1),
			)
		: width;
	for (const child of children) {
		if (child && typeof child === "object") {
			sizeUnitSpec(child as VegaLiteSpec, childWidth, 300);
		}
	}
}

function sizeFacetSpec(spec: VegaLiteSpec, width: number): void {
	const child = spec.spec;
	if (!child || typeof child !== "object") return;
	const facet = spec.facet as Record<string, unknown> | undefined;
	const childWidth = facet?.column
		? Math.min(300, width)
		: Math.max(320, width - 120);
	sizeUnitSpec(child as VegaLiteSpec, childWidth, 260);
}

function sizeRepeatSpec(spec: VegaLiteSpec, width: number): void {
	const child = spec.spec;
	if (!child || typeof child !== "object") return;
	const count = repeatCount(spec.repeat);
	const childWidth = Math.max(
		220,
		Math.min(340, (width - 24 * (count - 1)) / count),
	);
	sizeUnitSpec(child as VegaLiteSpec, childWidth, 260);
}

function sizeSingleSpec(
	spec: VegaLiteSpec,
	chartType: string,
	width: number,
): void {
	const circular = ["pie", "donut", "radial"].includes(chartType);
	const tall = ["parallel-coordinates", "ternary"].includes(chartType);
	let height = 300;
	if (circular) height = 340;
	else if (tall) height = 360;
	sizeUnitSpec(spec, circular ? Math.min(width, 480) : width, height);
}

export function sizeVegaSpec(
	source: VegaLiteSpec,
	chartType: string,
	containerWidth: number,
): VegaLiteSpec {
	const { title: _chartTitle, ...spec } = structuredClone(source);
	const width = Math.max(280, containerWidth - 8);

	if (chartType === "concat") sizeConcatSpec(spec, width);
	else if (chartType === "facet") sizeFacetSpec(spec, width);
	else if (chartType === "repeat") sizeRepeatSpec(spec, width);
	else sizeSingleSpec(spec, chartType, width);
	return spec;
}

export type ChartInteraction = {
	mode: "hover" | "click" | "brush" | "zoom" | "legend";
	label: string;
	resettable: boolean;
};

const interactions: Record<string, ChartInteraction> = {
	agent_hover: { mode: "hover", label: "Point to inspect", resettable: false },
	agent_select: {
		mode: "click",
		label: "Select a mark",
		resettable: true,
	},
	agent_brush: { mode: "brush", label: "Drag to select", resettable: true },
	agent_zoom: { mode: "zoom", label: "Drag to zoom", resettable: true },
	agent_legend: {
		mode: "legend",
		label: "Select a series",
		resettable: true,
	},
};

export function chartInteraction(value: unknown): ChartInteraction | null {
	if (Array.isArray(value)) {
		for (const item of value) {
			const result = chartInteraction(item);
			if (result) return result;
		}
		return null;
	}
	if (!value || typeof value !== "object") return null;
	const object = value as Record<string, unknown>;
	const name = typeof object.name === "string" ? object.name : "";
	if (interactions[name]) return interactions[name];
	for (const item of Object.values(object)) {
		const result = chartInteraction(item);
		if (result) return result;
	}
	return null;
}

export function attachDatasets(
	spec: VegaLiteSpec,
	datasetIds: string[],
	datasets: Record<string, Dataset>,
): VegaLiteSpec {
	const namedDatasets = Object.fromEntries(
		datasetIds.flatMap((id) => {
			const dataset = datasets[id];
			return dataset ? [[id, datasetRows(dataset)]] : [];
		}),
	);
	return {
		...spec,
		datasets: namedDatasets,
	};
}
