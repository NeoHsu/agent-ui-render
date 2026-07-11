import type { Config } from "vega-lite";

function token(
	style: CSSStyleDeclaration,
	name: string,
	fallback: string,
): string {
	return style.getPropertyValue(name).trim() || fallback;
}

export function vegaTheme(element: HTMLElement): Config {
	const style = getComputedStyle(element);
	const text = token(style, "--agent-text", "#111827");
	const muted = token(style, "--agent-muted", "#6b7280");
	const border = token(style, "--agent-chart-border", "#d1d5db");
	const chartBackground = token(style, "--agent-chart-bg", "transparent");
	const series = Array.from({ length: 6 }, (_, index) =>
		token(
			style,
			`--agent-series-${index + 1}`,
			["#2563eb", "#0891b2", "#7c3aed", "#ea580c", "#16a34a", "#dc2626"][index],
		),
	);

	return {
		view: { stroke: border, fill: chartBackground },
		axis: {
			domainColor: border,
			gridColor: border,
			labelColor: muted,
			titleColor: text,
		},
		legend: {
			labelColor: muted,
			titleColor: text,
		},
		title: {
			color: text,
			subtitleColor: muted,
		},
		range: { category: series },
	};
}
