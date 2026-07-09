import type { Column, Dataset, Metric, Primitive, ViewIntent } from "./types";

export type SemanticTone =
	| "critical"
	| "error"
	| "warning"
	| "success"
	| "info"
	| "muted";

const semanticTones = new Set<SemanticTone>([
	"critical",
	"error",
	"warning",
	"success",
	"info",
	"muted",
]);

const exactStatusTones = new Map<string, SemanticTone>([
	["brick", "critical"],
	["fatal", "critical"],
	["critical", "critical"],
	["p0", "critical"],
	["sev1", "critical"],
	["sev-1", "critical"],
	["failed", "error"],
	["failure", "error"],
	["error", "error"],
	["recoverfail", "error"],
	["blocked", "error"],
	["warning", "warning"],
	["warn", "warning"],
	["degraded", "warning"],
	["pending", "warning"],
	["risk", "warning"],
	["high", "warning"],
	["in progress", "warning"],
	["recovering", "warning"],
	["ok", "success"],
	["success", "success"],
	["succeeded", "success"],
	["resolved", "success"],
	["recovered", "success"],
	["healthy", "success"],
	["confirmed", "success"],
	["complete", "success"],
	["passed", "success"],
	["low", "success"],
	["supporting", "info"],
	["planned", "info"],
	["baseline", "info"],
	["staged", "info"],
	["medium", "info"],
	["neutral", "muted"],
	["n/a", "muted"],
	["unknown", "muted"],
]);

const statusLikeColumnPattern =
	/(?:status|state|result|severity|priority|level|confidence|outcome|health|phase)/i;

export function formatMetric(metric: Metric): string {
	const value = formatPrimitive(metric.value, metric.format);
	return metric.unit ? `${value} ${metric.unit}` : value;
}

export function formatCell(value: Primitive, column: Column): string {
	const rendered = formatPrimitive(value, column.type);
	return column.unit && value !== null
		? `${rendered} ${column.unit}`
		: rendered;
}

export function formatPrimitive(value: Primitive, format?: string): string {
	if (value === null) return "—";
	if (typeof value === "number") {
		if (format === "percent") return `${formatNumber(value * 100)}%`;
		return formatNumber(value);
	}
	return String(value);
}

function formatNumber(value: number): string {
	return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

export function safeClass(value: string): string {
	return value.replace(/[^a-z0-9-]/gi, "").toLowerCase() || "info";
}

export function isSemanticTone(value: string): value is SemanticTone {
	return semanticTones.has(value as SemanticTone);
}

export function semanticToneForText(value: string): SemanticTone | undefined {
	const text = value.trim();
	if (!text || text === "—") return undefined;
	const normalized = text
		.toLowerCase()
		.replace(/[()[\]{}]/g, "")
		.trim();
	const exact = exactStatusTones.get(normalized);
	if (exact) return exact;

	if (
		/\b(?:fatal|critical|brick|corrupt|unrecoverable|panic|outage|sev\s*-?\s*1|p0|p1)\b/i.test(
			text,
		)
	) {
		return "critical";
	}
	if (
		/\b(?:fail(?:ed|ure)?|error|exception|recoverfail|pg\s*13|incompatible|crash|broken|timeout|invalid|refused|denied)\b|(?:cannot|can't|won't\s+open|does(?:n't| not)\s+open|打不開|無法開啟|失敗|錯誤)/i.test(
			text,
		)
	) {
		return "error";
	}
	if (
		/\b(?:warn(?:ing)?|degrad(?:ed|ing)?|pending|risk|suspect|retry|staged|partial|recovering)\b/i.test(
			text,
		)
	) {
		return "warning";
	}
	if (
		/\b(?:ok|success|succeeded|resolved|recovered|healthy|confirmed|complete|passed|ready)\b/i.test(
			text,
		)
	) {
		return "success";
	}
	if (/\b(?:info|supporting|planned|baseline|neutral)\b/i.test(text)) {
		return "info";
	}
	return undefined;
}

export function statusBadgeToneForText(
	value: string,
	columnKey?: string,
): SemanticTone | undefined {
	const text = value.trim();
	if (!text || text === "—") return undefined;
	const normalized = text
		.toLowerCase()
		.replace(/[()[\]{}]/g, "")
		.trim();
	const exact = exactStatusTones.get(normalized);
	if (exact) return exact;

	const tone = semanticToneForText(text);
	if (!tone) return undefined;
	const isShort = text.length <= 36 && !/[。；：.;:]/.test(text);
	const isStatusColumn = columnKey
		? statusLikeColumnPattern.test(columnKey)
		: false;
	return isShort || isStatusColumn ? tone : undefined;
}

export function semanticTextClass(value: string): string {
	const tone = semanticToneForText(value);
	return tone ? `semantic semantic-${tone}` : "";
}

export function cellValueClass(value: Primitive, column: Column): string {
	const rendered = formatCell(value, column);
	const badgeTone = statusBadgeToneForText(rendered, column.key);
	if (badgeTone) return `status-badge status-${badgeTone}`;
	return semanticTextClass(rendered);
}

export function tableCellClass(value: Primitive, column: Column): string {
	const rendered = formatCell(value, column);
	const tone =
		statusBadgeToneForText(rendered, column.key) ??
		semanticToneForText(rendered);
	return tone ? `cell-${tone}` : "";
}

export function columnIndex(dataset: Dataset, key?: string): number {
	if (!key) return -1;
	return dataset.columns.findIndex((column) => column.key === key);
}

export function columnLabel(dataset: Dataset, key?: string): string {
	const column = dataset.columns.find((candidate) => candidate.key === key);
	return column?.label || key || "Value";
}

export function viewColumnKeys(view: ViewIntent): string[] {
	if (view.columns?.length) return uniqueStrings(view.columns);
	return uniqueStrings([
		view.x,
		...(view.dimensions ?? []),
		...(view.measures ?? []),
	]);
}

export function projectDatasetForView(
	dataset: Dataset,
	view: ViewIntent,
): Dataset {
	const indexes = viewColumnKeys(view)
		.map((key) => columnIndex(dataset, key))
		.filter((index) => index >= 0);
	if (!indexes.length) return dataset;

	return {
		columns: indexes.map((index) => dataset.columns[index]),
		rows: dataset.rows.map((row) => indexes.map((index) => row[index] ?? null)),
	};
}

function uniqueStrings(values: Array<string | undefined>): string[] {
	return [
		...new Set(values.filter((value): value is string => Boolean(value))),
	];
}

export function numericValue(row: Primitive[], index: number): number | null {
	if (index < 0 || index >= row.length) return null;
	const value = row[index];
	return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function firstNumericColumn(dataset: Dataset): string | undefined {
	return dataset.columns.find((column) =>
		["number", "currency", "percent"].includes(column.type || ""),
	)?.key;
}

export function measureKeys(dataset: Dataset, view: ViewIntent): string[] {
	const measures = view.measures?.length
		? view.measures
		: [firstNumericColumn(dataset)];
	return measures.filter((key): key is string => typeof key === "string");
}

export function extent(values: number[]): [number, number] {
	const min = Math.min(...values);
	const max = Math.max(...values);
	if (min === max) return [min - 1, max + 1];
	const pad = (max - min) * 0.08;
	return [min - pad, max + pad];
}

export function viewTitle(view: ViewIntent, index: number): string {
	const labels: Record<ViewIntent["intent"], string> = {
		overview: "Overview",
		precise_records: "Records",
		trend: "Trend",
		comparison: "Comparison",
		distribution: "Distribution",
		composition: "Composition",
		relationship: "Relationship",
	};
	return `${labels[view.intent] || "View"} ${index + 1}`;
}
