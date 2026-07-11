import {
	nextTick,
	onBeforeUnmount,
	readonly,
	shallowRef,
	watch,
	type ComputedRef,
	type ShallowRef,
} from "vue";
import { compile, type TopLevelSpec } from "vega-lite";
import {
	parse,
	View,
	type Loader,
	type Spec as VegaSpec,
	type TooltipHandler,
} from "vega";
import { vegaTheme } from "../vega-theme.js";

const rejectingLoader: Loader = {
	load: async (uri) =>
		Promise.reject(new Error(`External Vega resource blocked: ${uri}`)),
	sanitize: async (uri) =>
		Promise.reject(new Error(`External Vega link blocked: ${uri}`)),
	http: async (uri) =>
		Promise.reject(new Error(`External Vega HTTP request blocked: ${uri}`)),
	file: async (filename) =>
		Promise.reject(new Error(`External Vega file blocked: ${filename}`)),
};

export type VegaTooltipEntry = {
	label: string;
	value: string;
};

export type VegaTooltipState = {
	visible: boolean;
	x: number;
	y: number;
	entries: VegaTooltipEntry[];
};

function displayValue(value: unknown): string {
	if (value === null || value === undefined) return "—";
	if (value instanceof Date) return value.toLocaleString();
	if (Array.isArray(value)) return value.map(displayValue).join(", ");
	if (typeof value === "object") return JSON.stringify(value);
	return String(value);
}

function tooltipLabel(label: string): string {
	const normalized = label.replace(/^__/, "").replaceAll("_", " ").trim();
	if (!normalized) return "Value";
	return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

function tooltipEntries(value: unknown): VegaTooltipEntry[] {
	if (value && typeof value === "object" && !Array.isArray(value)) {
		return Object.entries(value).map(([label, item]) => ({
			label: tooltipLabel(label),
			value: displayValue(item),
		}));
	}
	return [{ label: "Value", value: displayValue(value) }];
}

function ariaTooltipEntries(label: string): VegaTooltipEntry[] {
	return label.split(";").map((part) => {
		const separator = part.indexOf(":");
		return separator < 0
			? { label: "Data", value: part.trim() }
			: {
					label: tooltipLabel(part.slice(0, separator).trim()),
					value: part.slice(separator + 1).trim(),
				};
	});
}

function createTooltipHandler(
	host: Readonly<ShallowRef<HTMLElement | null>>,
	tooltip: ShallowRef<VegaTooltipState>,
): TooltipHandler {
	return (_handler, event, _item, value) => {
		const element = host.value;
		if (!element || value === null || value === undefined) {
			tooltip.value = { ...tooltip.value, visible: false };
			return;
		}
		const bounds = element.getBoundingClientRect();
		tooltip.value = {
			visible: true,
			x:
				element.offsetLeft +
				Math.min(
					Math.max(12, event.clientX - bounds.left + 14),
					Math.max(12, bounds.width - 260),
				),
			y: element.offsetTop + Math.max(12, event.clientY - bounds.top + 14),
			entries: tooltipEntries(value),
		};
	};
}

function enrichLegendMarks(element: HTMLElement): void {
	const legendMarks = Array.from(
		element.querySelectorAll<SVGGraphicsElement>("g.role-legend-symbol > *"),
	);
	const legendLabels = Array.from(
		element.querySelectorAll<SVGGraphicsElement>("g.role-legend-label > *"),
	);
	legendMarks.forEach((mark, index) => {
		mark.setAttribute("role", "graphics-symbol");
		mark.setAttribute(
			"aria-label",
			`Legend item: ${legendLabels[index]?.textContent?.trim() || index + 1}`,
		);
	});
}

function setRovingMark(marks: SVGGraphicsElement[], index: number): void {
	const boundedIndex = Math.max(0, Math.min(index, marks.length - 1));
	marks.forEach((mark, markIndex) =>
		mark.setAttribute("tabindex", markIndex === boundedIndex ? "0" : "-1"),
	);
	marks[boundedIndex]?.focus();
}

function handleMarkKeyDown(
	event: KeyboardEvent,
	marks: SVGGraphicsElement[],
): void {
	const index = marks.indexOf(event.target as SVGGraphicsElement);
	if (index < 0) return;
	if (event.key === "ArrowRight" || event.key === "ArrowDown") {
		event.preventDefault();
		setRovingMark(marks, (index + 1) % marks.length);
	} else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
		event.preventDefault();
		setRovingMark(marks, (index - 1 + marks.length) % marks.length);
	} else if (event.key === "Home" || event.key === "End") {
		event.preventDefault();
		setRovingMark(marks, event.key === "Home" ? 0 : marks.length - 1);
	} else if (event.key === "Enter" || event.key === " ") {
		event.preventDefault();
		marks[index]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
	}
}

function showFocusedMark(
	event: FocusEvent,
	element: HTMLElement,
	marks: SVGGraphicsElement[],
	tooltip: ShallowRef<VegaTooltipState>,
): void {
	const mark = event.target as SVGGraphicsElement;
	const index = marks.indexOf(mark);
	if (index < 0) return;
	marks.forEach((item, itemIndex) =>
		item.setAttribute("tabindex", itemIndex === index ? "0" : "-1"),
	);
	const label = mark.getAttribute("aria-label");
	if (!label) return;
	const hostBounds = element.getBoundingClientRect();
	const markBounds = mark.getBoundingClientRect();
	tooltip.value = {
		visible: true,
		x:
			element.offsetLeft +
			Math.min(
				Math.max(12, markBounds.right - hostBounds.left + 12),
				Math.max(12, hostBounds.width - 260),
			),
		y: element.offsetTop + Math.max(12, markBounds.top - hostBounds.top),
		entries: ariaTooltipEntries(label),
	};
}

function installKeyboardInteractions(
	element: HTMLElement,
	tooltip: ShallowRef<VegaTooltipState>,
): () => void {
	enrichLegendMarks(element);
	const marks = Array.from(
		element.querySelectorAll<SVGGraphicsElement>(
			'g.role-mark > [role="graphics-symbol"], g.role-legend-symbol > [role="graphics-symbol"]',
		),
	);
	if (marks.length === 0) return () => undefined;
	element.setAttribute("tabindex", "-1");
	marks.forEach((mark, index) =>
		mark.setAttribute("tabindex", index === 0 ? "0" : "-1"),
	);
	const onKeyDown = (event: KeyboardEvent): void =>
		handleMarkKeyDown(event, marks);
	const onFocusIn = (event: FocusEvent): void =>
		showFocusedMark(event, element, marks, tooltip);
	const onFocusOut = (event: FocusEvent): void => {
		if (!marks.includes(event.relatedTarget as SVGGraphicsElement)) {
			tooltip.value = { ...tooltip.value, visible: false };
		}
	};
	element.addEventListener("keydown", onKeyDown);
	element.addEventListener("focusin", onFocusIn);
	element.addEventListener("focusout", onFocusOut);
	return () => {
		element.removeEventListener("keydown", onKeyDown);
		element.removeEventListener("focusin", onFocusIn);
		element.removeEventListener("focusout", onFocusOut);
		element.setAttribute("tabindex", "0");
	};
}

export function useVegaLiteView(
	host: Readonly<ShallowRef<HTMLElement | null>>,
	spec: ComputedRef<TopLevelSpec>,
) {
	const activeView = shallowRef<View | null>(null);
	const error = shallowRef<string | null>(null);
	const tooltip = shallowRef<VegaTooltipState>({
		visible: false,
		x: 0,
		y: 0,
		entries: [],
	});
	let generation = 0;
	let removeKeyboardInteractions: (() => void) | null = null;

	const handleTooltip = createTooltipHandler(host, tooltip);

	async function render(): Promise<void> {
		const currentGeneration = ++generation;
		removeKeyboardInteractions?.();
		removeKeyboardInteractions = null;
		activeView.value?.finalize();
		activeView.value = null;
		error.value = null;
		tooltip.value = { ...tooltip.value, visible: false };
		await nextTick();
		if (!host.value || currentGeneration !== generation) return;

		try {
			const compiled = compile(spec.value, {
				config: vegaTheme(host.value),
			}).spec as VegaSpec;
			const view = new View(parse(compiled), {
				renderer: "svg",
				loader: rejectingLoader,
				hover: true,
				tooltip: handleTooltip,
			}).initialize(host.value);
			activeView.value = view;
			await view.runAsync();
			if (currentGeneration !== generation) {
				view.finalize();
			} else if (host.value) {
				removeKeyboardInteractions = installKeyboardInteractions(
					host.value,
					tooltip,
				);
			}
		} catch (cause) {
			if (currentGeneration !== generation) return;
			activeView.value?.finalize();
			activeView.value = null;
			error.value = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function resetInteraction(): Promise<void> {
		await render();
	}

	watch(spec, render, { immediate: true, flush: "post" });

	onBeforeUnmount(() => {
		generation += 1;
		removeKeyboardInteractions?.();
		removeKeyboardInteractions = null;
		activeView.value?.finalize();
		activeView.value = null;
		tooltip.value = { ...tooltip.value, visible: false };
	});

	return {
		error: readonly(error),
		tooltip: readonly(tooltip),
		rerender: render,
		resetInteraction,
	};
}
