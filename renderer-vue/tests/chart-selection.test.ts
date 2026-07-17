import { describe, expect, it } from "vitest";
import {
	barOrientationForView,
	chartKindForView,
} from "../src/chart-selection.js";
import type { Dataset, ViewIntent } from "../src/types.js";

const temporalDataset: Dataset = {
	columns: [
		{ key: "period", label: "Period", type: "string" },
		{ key: "revenue", label: "Revenue", type: "currency", unit: "USD" },
		{ key: "profit", label: "Profit", type: "currency", unit: "USD" },
	],
	rows: [
		["Q1", 100, 20],
		["Q2", 120, 30],
		["Q3", 140, 35],
	],
};

function view(
	intent: ViewIntent["intent"],
	overrides: Partial<ViewIntent> = {},
): ViewIntent {
	return {
		intent,
		data: "report",
		x: "period",
		measures: ["revenue"],
		...overrides,
	};
}

describe("chart selection", () => {
	it("maps semantic trend and relationship intents", () => {
		expect(chartKindForView(view("trend"), temporalDataset)).toBe("line");
		expect(
			chartKindForView(
				view("relationship", { x: "revenue", measures: ["profit"] }),
				temporalDataset,
			),
		).toBe("scatter");
	});

	it("uses pie only for a small positive composition", () => {
		expect(chartKindForView(view("composition"), temporalDataset)).toBe("pie");

		const tooManyCategories: Dataset = {
			...temporalDataset,
			rows: Array.from({ length: 6 }, (_, index) => [
				`Category ${index + 1}`,
				index + 1,
				index + 1,
			]),
		};
		expect(chartKindForView(view("composition"), tooManyCategories)).toBe(
			"bar",
		);
	});

	it("uses vertical bars only for compact temporal categories with compatible measures", () => {
		const comparison = view("comparison", {
			measures: ["revenue", "profit"],
		});
		expect(barOrientationForView(comparison, temporalDataset)).toBe("vertical");

		const incompatibleDataset: Dataset = {
			...temporalDataset,
			columns: [
				...temporalDataset.columns,
				{ key: "growth", label: "Growth", type: "percent" },
			],
			rows: temporalDataset.rows.map((row) => [...row, 0.1]),
		};
		expect(
			barOrientationForView(
				view("comparison", { measures: ["revenue", "growth"] }),
				incompatibleDataset,
			),
		).toBe("horizontal");
	});
});
