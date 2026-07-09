export type Primitive = string | number | boolean | null;

export type Column = {
	key: string;
	label?: string;
	type?:
		| "string"
		| "number"
		| "currency"
		| "percent"
		| "date"
		| "datetime"
		| "boolean";
	unit?: string;
	description?: string;
};

export type Dataset = {
	columns: Column[];
	rows: Primitive[][];
};

export type Metric = {
	label: string;
	value: Primitive;
	format?: "number" | "currency" | "percent" | "string";
	unit?: string;
	delta?: {
		value: number;
		format?: "number" | "percent";
		direction?: "up" | "down" | "flat";
		label?: string;
	};
};

export type UITheme = "report-light" | "technical-dark" | "executive-clean";
export type UIDensity = "comfortable" | "compact";
export type UIEmphasis = "strong" | "subtle";

export type Alert = {
	level: "info" | "success" | "warning" | "error" | "critical";
	title?: string;
	content: string;
};

export type MarkdownSection = {
	title?: string;
	content: string;
};

export type ViewIntent = {
	intent:
		| "overview"
		| "precise_records"
		| "trend"
		| "comparison"
		| "distribution"
		| "composition"
		| "relationship";
	data: string;
	x?: string;
	measures?: string[];
	dimensions?: string[];
	columns?: string[];
	priority?: "high" | "medium" | "low";
	title?: string;
};

export type Report = {
	schema: "ui.input.normalized";
	version: 1;
	title?: string;
	summary?: string;
	theme?: UITheme;
	density?: UIDensity;
	emphasis?: UIEmphasis;
	datasets?: Record<string, Dataset>;
	metrics?: Metric[];
	insights?: string[];
	markdown?: MarkdownSection[];
	views?: ViewIntent[];
	alerts?: Alert[];
	assumptions?: string[];
};
