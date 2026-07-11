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
	const primary = token(style, "--agent-primary", "#2563eb");
	const border = token(style, "--agent-chart-border", "#d1d5db");
	const grid = token(style, "--agent-chart-grid", border);
	const chartBackground = token(style, "--agent-chart-bg", "transparent");
	const font = style.fontFamily || "Inter, ui-sans-serif, system-ui, sans-serif";
	const series = Array.from({ length: 6 }, (_, index) =>
		token(
			style,
			`--agent-series-${index + 1}`,
			["#2563eb", "#0891b2", "#7c3aed", "#ea580c", "#16a34a", "#dc2626"][index],
		),
	);

	return {
		background: chartBackground,
		font,
		view: { stroke: null, fill: chartBackground },
		axis: {
			domain: false,
			ticks: false,
			gridColor: grid,
			gridOpacity: 0.72,
			gridWidth: 1,
			labelColor: muted,
			labelFontSize: 11,
			labelPadding: 8,
			titleColor: text,
			titleFontSize: 11,
			titleFontWeight: 600,
			titlePadding: 12,
		},
		legend: {
			direction: "horizontal",
			labelColor: muted,
			labelFontSize: 11,
			labelLimit: 180,
			orient: "top",
			padding: 4,
			symbolSize: 74,
			symbolStrokeWidth: 1.5,
			titleColor: text,
			titleFontSize: 11,
			titleFontWeight: 600,
		},
		title: {
			anchor: "start",
			color: text,
			fontSize: 13,
			fontWeight: 600,
			offset: 12,
			subtitleColor: muted,
			subtitleFontSize: 11,
		},
		line: { strokeWidth: 2.25 },
		point: { filled: true, size: 72 },
		bar: { cornerRadiusEnd: 4 },
		selection: {
			interval: {
				mark: {
					fill: primary,
					fillOpacity: 0.1,
					stroke: primary,
					strokeOpacity: 0.9,
					strokeWidth: 1.5,
				},
			},
		},
		range: { category: series },
	};
}
