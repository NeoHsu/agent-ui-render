import type { Dataset, ViewIntent } from "./types.js";
import {
	columnIndex,
	columnLabel,
	extent,
	formatCell,
	measureKeys,
	numericValue,
} from "./format.js";

// Single home for chart view-model computation. Components only render these
// models, which keeps formatting, geometry, and palette decisions consistent.

export const seriesColors = [
	"var(--agent-series-1)",
	"var(--agent-series-2)",
	"var(--agent-series-3)",
	"var(--agent-series-4)",
	"var(--agent-series-5)",
	"var(--agent-series-6)",
];

export const pieRadius = 92;
export const pieCircumference = 2 * Math.PI * pieRadius;

// SVG plot geometry shared by line and scatter charts (760x300 viewBox).
const plotLeft = 72;
const plotTop = 32;
const plotWidth = 644;
const plotHeight = 190;
const tickCount = 5;

export type AxisTick = {
	key: string;
	position: string;
	label: string;
};

export function chartAriaLabel(view: ViewIntent): string {
	return `${view.title || view.intent} chart for ${view.data}`;
}

export type LinePoint = {
	key: string;
	cx: string;
	cy: string;
	label: string;
	title: string;
};

export type LineSeries = {
	key: string;
	label: string;
	points: string;
	color: string;
	markers: LinePoint[];
};

export type LineChartModel = {
	key: string;
	title?: string;
	series: LineSeries[];
	xTicks: AxisTick[];
	yTicks: AxisTick[];
	showPointLabels: boolean;
};

export function lineChartModels(
	dataset: Dataset,
	view: ViewIntent,
): LineChartModel[] {
	const keys = measureKeys(dataset, view).slice(0, 3);
	if (!keys.length) return [];

	const signatures = new Set(
		keys.map((key) => {
			const column = dataset.columns[columnIndex(dataset, key)];
			return `${column?.type ?? ""}:${column?.unit ?? ""}`;
		}),
	);
	const groups = signatures.size > 1 ? keys.map((key) => [key]) : [keys];
	return groups.flatMap((group, index) => {
		const model = buildLineChartModel(dataset, view, group, index);
		if (!model) return [];
		return [
			{
				...model,
				title: groups.length > 1 ? model.series[0]?.label : undefined,
			},
		];
	});
}

function buildLineChartModel(
	dataset: Dataset,
	view: ViewIntent,
	keys: string[],
	groupIndex: number,
): LineChartModel | null {
	const allValues: number[] = [];
	for (const key of keys) {
		const index = columnIndex(dataset, key);
		for (const row of dataset.rows) {
			const value = numericValue(row, index);
			if (value !== null) allValues.push(value);
		}
	}
	if (!allValues.length) return null;

	const [minY, maxY] = extent(allValues);
	const maxPos = Math.max(1, dataset.rows.length - 1);
	const xIndex = columnIndex(dataset, view.x);
	const xColumn = dataset.columns[xIndex];
	const yColumn = dataset.columns[columnIndex(dataset, keys[0])];
	const series = keys.map((key, seriesIndex) => {
		const index = columnIndex(dataset, key);
		const column = dataset.columns[index];
		const markers = dataset.rows
			.map((row, rowIndex): LinePoint | null => {
				const value = numericValue(row, index);
				if (value === null) return null;
				const x = plotLeft + (rowIndex / maxPos) * plotWidth;
				const y = plotTop + (1 - (value - minY) / (maxY - minY)) * plotHeight;
				const valueLabel = column ? formatCell(value, column) : String(value);
				const category =
					xIndex >= 0 && xColumn
						? formatCell(row[xIndex] ?? null, xColumn)
						: `Point ${rowIndex + 1}`;
				return {
					key: `${key}-${rowIndex}`,
					cx: x.toFixed(1),
					cy: y.toFixed(1),
					label: valueLabel,
					title: `${category}: ${valueLabel}`,
				};
			})
			.filter((point): point is LinePoint => point !== null);
		return {
			key,
			label: columnLabel(dataset, key),
			points: markers.map((point) => `${point.cx},${point.cy}`).join(" "),
			color: seriesColor(groupIndex + seriesIndex),
			markers,
		};
	});

	return {
		key: `${keys.join("-")}-${groupIndex}`,
		series,
		xTicks: dataset.rows.map((row, rowIndex) => ({
			key: `x-${rowIndex}`,
			position: (plotLeft + (rowIndex / maxPos) * plotWidth).toFixed(1),
			label:
				xIndex >= 0 && xColumn
					? formatCell(row[xIndex] ?? null, xColumn)
					: String(rowIndex + 1),
		})),
		yTicks: axisTicks(minY, maxY, yColumn, "y"),
		showPointLabels: series.length === 1 && dataset.rows.length <= 8,
	};
}

export type ScatterPoint = {
	key: string;
	cx: string;
	cy: string;
	label: string;
	title: string;
};

export type ScatterChartModel = {
	label: string;
	xLabel: string;
	yLabel: string;
	xTicks: AxisTick[];
	yTicks: AxisTick[];
	points: ScatterPoint[];
};

export function scatterChartModel(
	dataset: Dataset,
	view: ViewIntent,
): ScatterChartModel {
	const empty: ScatterChartModel = {
		label: "",
		xLabel: "",
		yLabel: "",
		xTicks: [],
		yTicks: [],
		points: [],
	};
	const xIndex = columnIndex(dataset, view.x);
	const measure = measureKeys(dataset, view)[0];
	const yIndex = columnIndex(dataset, measure);
	if (xIndex < 0 || yIndex < 0) return empty;

	const labelIndex = dataset.columns.findIndex(
		(column, index) =>
			index !== xIndex && index !== yIndex && column.type === "string",
	);
	const rawPoints = dataset.rows
		.map((row, rowIndex) => ({
			key: `${rowIndex}`,
			label: labelIndex >= 0 ? String(row[labelIndex] ?? "") : "",
			xValue: numericValue(row, xIndex),
			yValue: numericValue(row, yIndex),
		}))
		.filter(
			(
				point,
			): point is {
				key: string;
				label: string;
				xValue: number;
				yValue: number;
			} => point.xValue !== null && point.yValue !== null,
		);
	if (!rawPoints.length) return empty;

	const [minX, maxX] = extent(rawPoints.map((point) => point.xValue));
	const [minY, maxY] = extent(rawPoints.map((point) => point.yValue));
	const xColumn = dataset.columns[xIndex];
	const yColumn = dataset.columns[yIndex];
	return {
		label: `${columnLabel(dataset, measure)} by ${columnLabel(dataset, view.x)}`,
		xLabel: columnLabel(dataset, view.x),
		yLabel: columnLabel(dataset, measure),
		xTicks: axisTicks(minX, maxX, xColumn, "x"),
		yTicks: axisTicks(minY, maxY, yColumn, "y"),
		points: rawPoints.map((point) => ({
			key: point.key,
			label: point.label,
			cx: (
				plotLeft +
				((point.xValue - minX) / (maxX - minX)) * plotWidth
			).toFixed(1),
			cy: (
				plotTop +
				(1 - (point.yValue - minY) / (maxY - minY)) * plotHeight
			).toFixed(1),
			title: `${point.label ? `${point.label}: ` : ""}${
				xColumn ? formatCell(point.xValue, xColumn) : point.xValue
			} → ${yColumn ? formatCell(point.yValue, yColumn) : point.yValue}`,
		})),
	};
}

export type PieSlice = {
	key: string;
	label: string;
	value: number;
	color: string;
	dashArray: string;
	dashOffset: string;
	valueText: string;
	percentText: string;
};

export type PieChartModel = {
	totalLabel: string;
	slices: PieSlice[];
};

export function pieChartModel(
	dataset: Dataset,
	view: ViewIntent,
): PieChartModel {
	const empty: PieChartModel = { slices: [], totalLabel: "—" };
	if (view.intent !== "composition") return empty;
	const xIndex = columnIndex(dataset, view.x);
	const measure = measureKeys(dataset, view)[0];
	const yIndex = columnIndex(dataset, measure);
	if (xIndex < 0 || yIndex < 0) return empty;

	const measureColumn = dataset.columns[yIndex];
	const rawSlices = dataset.rows
		.map((row, rowIndex) => ({
			key: `${rowIndex}`,
			label: String(row[xIndex] ?? ""),
			value: numericValue(row, yIndex) ?? 0,
		}))
		.filter((item) => item.value > 0);
	const total = rawSlices.reduce((sum, item) => sum + item.value, 0);
	if (total <= 0) return empty;

	let offset = 0;
	const slices = rawSlices.map((item, index) => {
		const length = (item.value / total) * pieCircumference;
		const dashOffset = -offset;
		offset += length;
		return {
			...item,
			color: seriesColor(index),
			dashArray: `${length.toFixed(2)} ${pieCircumference.toFixed(2)}`,
			dashOffset: dashOffset.toFixed(2),
			valueText: measureColumn
				? formatCell(item.value, measureColumn)
				: String(item.value),
			percentText: formatSharePercent(item.value / total),
		};
	});

	return {
		slices,
		totalLabel: measureColumn
			? formatCell(total, measureColumn)
			: String(total),
	};
}

export type ChartLegendItem = {
	key: string;
	label: string;
	color: string;
};

export type BarSeries = ChartLegendItem & {
	value: string;
	width: string;
	labelPlacement: "inside" | "outside";
};

export type BarGroup = {
	key: string;
	label: string;
	series: BarSeries[];
};

export type BarChartModel = {
	groups: BarGroup[];
	legend: ChartLegendItem[];
	axisStart: string;
	axisEnd: string;
	sharedScale: boolean;
};

export function barChartModel(
	dataset: Dataset,
	view: ViewIntent,
): BarChartModel {
	const empty: BarChartModel = {
		groups: [],
		legend: [],
		axisStart: "0",
		axisEnd: "",
		sharedScale: true,
	};
	const xIndex = columnIndex(dataset, view.x);
	const keys = measureKeys(dataset, view).slice(0, 3);
	if (!keys.length) return empty;

	const columns = keys.map((key) => dataset.columns[columnIndex(dataset, key)]);
	const signatures = new Set(
		columns.map((column) => `${column?.type ?? ""}:${column?.unit ?? ""}`),
	);
	const globalValues = numericValuesForKeys(dataset, keys);
	if (!globalValues.length) return empty;

	const largestValue = Math.max(...globalValues);
	const globalMax = largestValue > 0 ? largestValue : 1;
	const sharedScale = signatures.size === 1;
	const maxima = keys.map((key) =>
		seriesMaximum(dataset, key, globalMax, sharedScale),
	);
	const legend = chartLegend(dataset, keys);
	const context = { dataset, keys, maxima, legend };
	return {
		groups: horizontalBarGroups(context, xIndex),
		legend,
		axisStart: formatAxisValue(0, columns[0]),
		axisEnd: sharedScale
			? formatAxisValue(globalMax, columns[0])
			: "Per-series scale",
		sharedScale,
	};
}

type HorizontalBarContext = {
	dataset: Dataset;
	keys: string[];
	maxima: number[];
	legend: ChartLegendItem[];
};

function horizontalBarGroups(
	context: HorizontalBarContext,
	xIndex: number,
): BarGroup[] {
	return context.dataset.rows.map((row, rowIndex) => ({
		key: `${rowIndex}`,
		label: xIndex >= 0 ? String(row[xIndex] ?? "") : "",
		series: horizontalBarSeries(context, row),
	}));
}

function horizontalBarSeries(
	context: HorizontalBarContext,
	row: Dataset["rows"][number],
): BarSeries[] {
	return context.keys.map((key, seriesIndex) => {
		const index = columnIndex(context.dataset, key);
		const column = context.dataset.columns[index];
		const value = Math.max(0, numericValue(row, index) ?? 0);
		const width = Math.max(
			2,
			(value / (context.maxima[seriesIndex] ?? 1)) * 100,
		);
		const renderedValue = column ? formatCell(value, column) : String(value);
		const legendItem = context.legend[seriesIndex] ?? {
			key,
			label: columnLabel(context.dataset, key),
			color: seriesColor(seriesIndex),
		};
		return {
			...legendItem,
			value: renderedValue,
			width: `${width.toFixed(1)}%`,
			labelPlacement: canFitBarLabel(width, renderedValue)
				? "inside"
				: "outside",
		};
	});
}

function formatAxisValue(
	value: number,
	column: Dataset["columns"][number] | undefined,
): string {
	return column ? formatCell(value, column) : String(value);
}

export type VerticalBar = {
	key: string;
	label: string;
	value: string;
	color: string;
	x: string;
	y: string;
	width: string;
	height: string;
	labelX: string;
	labelY: string;
};

export type VerticalBarGroup = {
	key: string;
	label: string;
	labelX: string;
	bars: VerticalBar[];
};

export type VerticalBarChartModel = {
	groups: VerticalBarGroup[];
	legend: ChartLegendItem[];
	yTicks: AxisTick[];
};

export function verticalBarChartModel(
	dataset: Dataset,
	view: ViewIntent,
): VerticalBarChartModel {
	const empty: VerticalBarChartModel = { groups: [], legend: [], yTicks: [] };
	const xIndex = columnIndex(dataset, view.x);
	const keys = measureKeys(dataset, view).slice(0, 3);
	if (xIndex < 0 || !keys.length || !dataset.rows.length) return empty;
	const values = numericValuesForKeys(dataset, keys).map((value) =>
		Math.max(0, value),
	);
	const maximum = Math.max(...values, 0);
	if (maximum <= 0) return empty;

	const chartMaximum = maximum * 1.18;
	const groupStep = plotWidth / dataset.rows.length;
	const clusterWidth = Math.min(groupStep * 0.7, 118);
	const gap = keys.length > 1 ? 4 : 0;
	const barWidth = (clusterWidth - gap * (keys.length - 1)) / keys.length;
	const xColumn = dataset.columns[xIndex];
	const valueColumn = dataset.columns[columnIndex(dataset, keys[0])];
	const legend = chartLegend(dataset, keys);
	return {
		groups: dataset.rows.map((row, rowIndex) => {
			const center = plotLeft + groupStep * (rowIndex + 0.5);
			const start = center - clusterWidth / 2;
			return {
				key: `${rowIndex}`,
				label: xColumn
					? formatCell(row[xIndex] ?? null, xColumn)
					: String(row[xIndex] ?? ""),
				labelX: center.toFixed(1),
				bars: keys.map((key, seriesIndex) => {
					const index = columnIndex(dataset, key);
					const column = dataset.columns[index];
					const value = Math.max(0, numericValue(row, index) ?? 0);
					const height = (value / chartMaximum) * plotHeight;
					const y = plotTop + plotHeight - height;
					const x = start + seriesIndex * (barWidth + gap);
					return {
						key,
						label: columnLabel(dataset, key),
						value: column ? formatCell(value, column) : String(value),
						color: seriesColor(seriesIndex),
						x: x.toFixed(1),
						y: y.toFixed(1),
						width: Math.max(2, barWidth).toFixed(1),
						height: Math.max(0, height).toFixed(1),
						labelX: (x + barWidth / 2).toFixed(1),
						labelY: Math.max(plotTop + 9, y - 6).toFixed(1),
					};
				}),
			};
		}),
		legend,
		yTicks: axisTicks(0, chartMaximum, valueColumn, "y"),
	};
}

function numericValuesForKeys(dataset: Dataset, keys: string[]): number[] {
	return keys.flatMap((key) => {
		const index = columnIndex(dataset, key);
		return dataset.rows
			.map((row) => numericValue(row, index))
			.filter((value): value is number => value !== null);
	});
}

function seriesMaximum(
	dataset: Dataset,
	key: string,
	globalMaximum: number,
	sharedScale: boolean,
): number {
	if (sharedScale) return globalMaximum;
	const index = columnIndex(dataset, key);
	const maximum = Math.max(
		...dataset.rows.map((row) => numericValue(row, index) ?? 0),
	);
	return maximum > 0 ? maximum : 1;
}

function chartLegend(dataset: Dataset, keys: string[]): ChartLegendItem[] {
	return keys.map((key, index) => ({
		key,
		label: columnLabel(dataset, key),
		color: seriesColor(index),
	}));
}

function canFitBarLabel(widthPercent: number, label: string): boolean {
	return widthPercent >= Math.max(12, label.length * 1.55);
}

function axisTicks(
	min: number,
	max: number,
	column: Dataset["columns"][number] | undefined,
	axis: "x" | "y",
): AxisTick[] {
	return Array.from({ length: tickCount }, (_, index) => {
		const ratio = index / (tickCount - 1);
		const value = min + ratio * (max - min);
		const position =
			axis === "x"
				? plotLeft + ratio * plotWidth
				: plotTop + (1 - ratio) * plotHeight;
		return {
			key: `${axis}-${index}`,
			position: position.toFixed(1),
			label: column ? formatCell(value, column) : value.toFixed(1),
		};
	});
}

function seriesColor(index: number): string {
	return seriesColors[index % seriesColors.length] ?? "var(--agent-series-1)";
}

function formatSharePercent(value: number): string {
	const percent = value * 100;
	return Number.isInteger(percent) ? `${percent}%` : `${percent.toFixed(1)}%`;
}
