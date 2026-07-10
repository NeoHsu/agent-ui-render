import type { Dataset, ViewIntent } from "./types.js";

// Keep behavior in sync with crates/agent-ui-render-core/src/chart/mod.rs; this copy is shipped
// with generated Vue SFC handoff artifacts.
export type ChartKind = "line" | "bar" | "area" | "scatter" | "pie";
export type BarOrientation = "horizontal" | "vertical";

export const MAX_PIE_CATEGORIES = 5;

export function chartKindForView(
	view: ViewIntent,
	dataset: Dataset,
): ChartKind {
	if (view.intent === "trend") return "line";
	if (view.intent === "relationship") return "scatter";
	if (view.intent === "composition" && canUsePieChart(dataset, view)) {
		return "pie";
	}
	return "bar";
}

export function barOrientationForView(
	view: ViewIntent,
	dataset: Dataset,
): BarOrientation {
	if (
		view.intent !== "comparison" ||
		!hasCompactTemporalCategories(dataset, view)
	) {
		return "horizontal";
	}
	return hasCompatibleMeasures(dataset, view) ? "vertical" : "horizontal";
}

function hasCompactTemporalCategories(
	dataset: Dataset,
	view: ViewIntent,
): boolean {
	if (dataset.rows.length < 2 || dataset.rows.length > 8) return false;
	const xIndex = columnIndex(dataset, view.x);
	if (xIndex < 0) return false;
	const columnType = dataset.columns[xIndex]?.type;
	if (columnType === "date" || columnType === "datetime") return true;
	return dataset.rows.every((row) => isTemporalCategory(row[xIndex]));
}

function hasCompatibleMeasures(dataset: Dataset, view: ViewIntent): boolean {
	const signatures = new Set(
		measureKeys(dataset, view).map((key) => {
			const column = dataset.columns[columnIndex(dataset, key)];
			return `${column?.type ?? ""}:${column?.unit ?? ""}`;
		}),
	);
	return signatures.size === 1;
}

function isTemporalCategory(value: unknown): boolean {
	if (typeof value !== "string" && typeof value !== "number") return false;
	const text = String(value).trim();
	return (
		/^(?:Q[1-4]|[1-4]Q)(?:[\s'’/-]*\d{2,4})?$/i.test(text) ||
		/^(?:19|20)\d{2}$/.test(text) ||
		/^(?:19|20)\d{2}[-/]\d{1,2}(?:[-/]\d{1,2})?$/.test(text) ||
		/^(?:W(?:eek)?\s*\d{1,2})(?:[\s,/-]+\d{2,4})?$/i.test(text) ||
		/^(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:t(?:ember)?)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)(?:[\s,/-]+\d{2,4})?$/i.test(
			text,
		)
	);
}

function canUsePieChart(
	dataset: Dataset,
	view: ViewIntent,
	maxCategories = MAX_PIE_CATEGORIES,
): boolean {
	if (view.intent !== "composition") return false;
	const xIndex = columnIndex(dataset, view.x);
	const measure = measureKeys(dataset, view)[0];
	const yIndex = columnIndex(dataset, measure);
	if (xIndex < 0 || yIndex < 0 || dataset.rows.length === 0) return false;

	const distinctCategories = new Set(
		dataset.rows.map((row) => JSON.stringify(row[xIndex] ?? null)),
	).size;
	if (distinctCategories === 0 || distinctCategories > maxCategories) {
		return false;
	}

	const total = dataset.rows.reduce(
		(sum, row) => sum + Math.max(0, numericValue(row, yIndex) ?? 0),
		0,
	);
	return total > 0;
}

function columnIndex(dataset: Dataset, key?: string): number {
	if (!key) return -1;
	return dataset.columns.findIndex((column) => column.key === key);
}

function firstNumericColumn(dataset: Dataset): string | undefined {
	return dataset.columns.find((column) =>
		["number", "currency", "percent"].includes(column.type ?? ""),
	)?.key;
}

function measureKeys(dataset: Dataset, view: ViewIntent): string[] {
	const measures = view.measures?.length
		? view.measures
		: [firstNumericColumn(dataset)];
	return measures.filter((key): key is string => typeof key === "string");
}

function numericValue(row: unknown[], index: number): number | null {
	if (index < 0 || index >= row.length) return null;
	const value = row[index];
	return typeof value === "number" && Number.isFinite(value) ? value : null;
}
