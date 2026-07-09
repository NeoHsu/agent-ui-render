import type { Dataset, ViewIntent } from "./types";
import {
	columnIndex,
	columnLabel,
	extent,
	formatCell,
	measureKeys,
	numericValue,
} from "./format";

// Single home for chart view-model computation. The Vue client preview,
// Vue SFC handoff, and static HTML renderer consume equivalent models and emit
// markup only, so geometry, palette, and formatting cannot drift between outputs.

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

// SVG plot geometry shared by line and scatter charts (760x280 viewBox).
const plotLeft = 54;
const plotTop = 22;
const plotWidth = 682;
const plotHeight = 204;

export function chartAriaLabel(view: ViewIntent): string {
	return `${view.title || view.intent} chart for ${view.data}`;
}

export type LineSeries = {
	key: string;
	label: string;
	points: string;
	color: string;
};

export function lineChartModel(
	dataset: Dataset,
	view: ViewIntent,
): LineSeries[] {
	const keys = measureKeys(dataset, view).slice(0, 3);
	const allValues: number[] = [];
	for (const key of keys) {
		const index = columnIndex(dataset, key);
		for (const row of dataset.rows) {
			const value = numericValue(row, index);
			if (value !== null) allValues.push(value);
		}
	}
	if (!allValues.length) return [];

	const [minY, maxY] = extent(allValues);
	const maxPos = Math.max(1, dataset.rows.length - 1);
	return keys.map((key, seriesIndex) => {
		const index = columnIndex(dataset, key);
		const points = dataset.rows
			.map((row, rowIndex) => {
				const value = numericValue(row, index);
				if (value === null) return null;
				const x = plotLeft + (rowIndex / maxPos) * plotWidth;
				const y = plotTop + (1 - (value - minY) / (maxY - minY)) * plotHeight;
				return `${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.filter((point): point is string => point !== null)
			.join(" ");
		return {
			key,
			label: columnLabel(dataset, key),
			points,
			color: seriesColor(seriesIndex),
		};
	});
}

export type ScatterChartModel = {
	label: string;
	points: Array<{ key: string; cx: string; cy: string; title: string }>;
};

export function scatterChartModel(
	dataset: Dataset,
	view: ViewIntent,
): ScatterChartModel {
	const xIndex = columnIndex(dataset, view.x);
	const measure = measureKeys(dataset, view)[0];
	const yIndex = columnIndex(dataset, measure);
	if (xIndex < 0 || yIndex < 0) return { label: "", points: [] };

	const rawPoints = dataset.rows
		.map((row) => ({
			xValue: numericValue(row, xIndex),
			yValue: numericValue(row, yIndex),
		}))
		.filter(
			(point): point is { xValue: number; yValue: number } =>
				point.xValue !== null && point.yValue !== null,
		);
	if (!rawPoints.length) return { label: "", points: [] };

	const [minX, maxX] = extent(rawPoints.map((point) => point.xValue));
	const [minY, maxY] = extent(rawPoints.map((point) => point.yValue));
	const xColumn = dataset.columns[xIndex];
	const yColumn = dataset.columns[yIndex];
	return {
		label: `${columnLabel(dataset, measure)} by ${columnLabel(dataset, view.x)}`,
		points: rawPoints.map((point, index) => ({
			key: `${index}`,
			cx: (
				plotLeft +
				((point.xValue - minX) / (maxX - minX)) * plotWidth
			).toFixed(1),
			cy: (
				plotTop +
				(1 - (point.yValue - minY) / (maxY - minY)) * plotHeight
			).toFixed(1),
			title: `${xColumn ? formatCell(point.xValue, xColumn) : point.xValue} → ${
				yColumn ? formatCell(point.yValue, yColumn) : point.yValue
			}`,
		})),
	};
}

export type PieChartModel = {
	totalLabel: string;
	slices: Array<{
		key: string;
		label: string;
		value: number;
		color: string;
		dashArray: string;
		dashOffset: string;
		valueText: string;
		percentText: string;
	}>;
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

export type BarItem = { label: string; value: string; width: string };

export function barChartModel(
	dataset: Dataset,
	view: ViewIntent,
): BarItem[] {
	const xIndex = columnIndex(dataset, view.x);
	const measure = measureKeys(dataset, view)[0];
	const yIndex = columnIndex(dataset, measure);
	const values = dataset.rows
		.map((row) => numericValue(row, yIndex))
		.filter((value): value is number => value !== null);
	if (!values.length) return [];

	const max = Math.max(...values, 1);
	const measureColumn = dataset.columns[yIndex];
	return dataset.rows.map((row) => {
		const value = numericValue(row, yIndex) ?? 0;
		return {
			label: xIndex >= 0 ? String(row[xIndex] ?? "") : "",
			value: measureColumn ? formatCell(value, measureColumn) : String(value),
			width: `${Math.max(2, (value / max) * 100).toFixed(1)}%`,
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
