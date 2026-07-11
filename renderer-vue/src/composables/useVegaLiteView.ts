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
	anchorX: number;
	anchorY: number;
	title: string | null;
	color: string | null;
	entries: readonly VegaTooltipEntry[];
};

function emptyTooltipState(): VegaTooltipState {
	return {
		visible: false,
		x: 0,
		y: 0,
		anchorX: 0,
		anchorY: 0,
		title: null,
		color: null,
		entries: [],
	};
}

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
	return normalized
		.split(/\s+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ");
}

function tooltipEntryValue(label: string, value: unknown): string {
	if (/^(series|dimension)$/i.test(label) && typeof value === "string") {
		return tooltipLabel(value);
	}
	return displayValue(value);
}

function tooltipEntries(value: unknown): VegaTooltipEntry[] {
	if (value && typeof value === "object" && !Array.isArray(value)) {
		return Object.entries(value).map(([label, item]) => {
			const displayLabel = tooltipLabel(label);
			return {
				label: displayLabel,
				value: tooltipEntryValue(displayLabel, item),
			};
		});
	}
	return [{ label: "Value", value: displayValue(value) }];
}

function ariaTooltipEntries(label: string): VegaTooltipEntry[] {
	return label.split(";").map((part) => {
		const separator = part.indexOf(":");
		if (separator < 0) return { label: "Data", value: part.trim() };
		const displayLabel = tooltipLabel(part.slice(0, separator).trim());
		return {
			label: displayLabel,
			value: tooltipEntryValue(displayLabel, part.slice(separator + 1).trim()),
		};
	});
}

function tooltipContent(entries: VegaTooltipEntry[]): {
	title: string | null;
	entries: VegaTooltipEntry[];
} {
	if (entries.length === 1 && entries[0]?.label === "Legend Item") {
		return { title: entries[0].value, entries: [] };
	}
	const titleEntry = entries.find(
		(entry) => !/^(value|amount|count|measure|size)$/i.test(entry.label),
	);
	if (!titleEntry || entries.length === 1) return { title: null, entries };
	return {
		title: titleEntry.value,
		entries: entries.filter((entry) => entry !== titleEntry),
	};
}

function safeSceneColor(item: unknown): string | null {
	if (!item || typeof item !== "object") return null;
	const sceneItem = item as { fill?: unknown; stroke?: unknown };
	for (const candidate of [sceneItem.fill, sceneItem.stroke]) {
		if (
			typeof candidate === "string" &&
			(/^#[0-9a-f]{3,8}$/i.test(candidate) ||
				/^rgba?\([\d\s,.%]+\)$/i.test(candidate))
		) {
			return candidate;
		}
	}
	return null;
}

function createTooltipHandler(
	host: Readonly<ShallowRef<HTMLElement | null>>,
	tooltip: ShallowRef<VegaTooltipState>,
): TooltipHandler {
	return (_handler, event, item, value) => {
		const element = host.value;
		if (!element || value === null || value === undefined) {
			tooltip.value = { ...tooltip.value, visible: false };
			return;
		}
		const bounds = element.getBoundingClientRect();
		const anchorX = Math.max(
			0,
			Math.min(event.clientX - bounds.left, bounds.width),
		);
		const anchorY = Math.max(
			0,
			Math.min(event.clientY - bounds.top, bounds.height),
		);
		const content = tooltipContent(tooltipEntries(value));
		tooltip.value = {
			visible: true,
			x:
				element.offsetLeft +
				Math.min(Math.max(12, anchorX + 14), Math.max(12, bounds.width - 260)),
			y: element.offsetTop + Math.max(12, anchorY + 14),
			anchorX: element.offsetLeft + anchorX,
			anchorY: element.offsetTop + anchorY,
			title: content.title,
			color: safeSceneColor(item),
			entries: content.entries,
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
	const anchorX = markBounds.left - hostBounds.left + markBounds.width / 2;
	const anchorY = markBounds.top - hostBounds.top + markBounds.height / 2;
	const content = tooltipContent(ariaTooltipEntries(label));
	tooltip.value = {
		visible: true,
		x:
			element.offsetLeft +
			Math.min(
				Math.max(12, markBounds.right - hostBounds.left + 12),
				Math.max(12, hostBounds.width - 260),
			),
		y: element.offsetTop + Math.max(12, markBounds.top - hostBounds.top),
		anchorX: element.offsetLeft + anchorX,
		anchorY: element.offsetTop + anchorY,
		title: content.title,
		color: safeSceneColor({
			fill: mark.getAttribute("fill"),
			stroke: mark.getAttribute("stroke"),
		}),
		entries: content.entries,
	};
}

const persistentInteractionSignals = new Set([
	"agent_select",
	"agent_brush",
	"agent_zoom",
	"agent_legend",
]);

function interactionSignalNames(spec: VegaSpec): string[] {
	const signals =
		(spec as { signals?: Array<{ name?: unknown }> }).signals ?? [];
	return signals
		.map((signal) => signal.name)
		.filter(
			(name): name is string =>
				typeof name === "string" && persistentInteractionSignals.has(name),
		);
}

function interactionValueIsActive(value: unknown): boolean {
	if (value === null || value === undefined) return false;
	if (Array.isArray(value)) return value.length > 0;
	if (typeof value !== "object") return Boolean(value);
	return Object.values(value).some((item) =>
		Array.isArray(item)
			? item.length > 0 && item[0] !== item[1]
			: item !== null && item !== undefined,
	);
}

function compileTrustedSpec(
	spec: TopLevelSpec,
	element: HTMLElement,
): VegaSpec {
	return compile(spec, { config: vegaTheme(element) }).spec as VegaSpec;
}

function createTrustedView(
	compiled: VegaSpec,
	element: HTMLElement,
	tooltip: TooltipHandler,
	onInteraction: (active: boolean) => void,
): View {
	const view = new View(parse(compiled), {
		renderer: "svg",
		loader: rejectingLoader,
		hover: true,
		tooltip,
	}).initialize(element);
	for (const name of interactionSignalNames(compiled)) {
		view.addSignalListener(name, (_signal, value) => {
			onInteraction(interactionValueIsActive(value));
		});
	}
	return view;
}

function centeredZoomRange(
	value: unknown,
	size: number,
	factor: number,
): [number, number] {
	const range =
		Array.isArray(value) &&
		value.length === 2 &&
		typeof value[0] === "number" &&
		typeof value[1] === "number" &&
		value[0] !== value[1]
			? [value[0], value[1]]
			: [0, size];
	const center = (range[0] + range[1]) / 2;
	const span = Math.min(size, Math.max(8, (range[1] - range[0]) * factor));
	const start = Math.max(0, Math.min(size - span, center - span / 2));
	return [start, start + span];
}

async function zoomView(view: View, factor: number): Promise<void> {
	if (!Number.isFinite(factor) || factor <= 0) return;
	const width = Number(view.signal("width"));
	const height = Number(view.signal("height"));
	if (!(width > 0 && height > 0)) return;
	view.signal(
		"agent_zoom_x",
		centeredZoomRange(view.signal("agent_zoom_x"), width, factor),
	);
	view.signal(
		"agent_zoom_y",
		centeredZoomRange(view.signal("agent_zoom_y"), height, factor),
	);
	await view.runAsync();
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
	const interactionActive = shallowRef(false);
	const tooltip = shallowRef<VegaTooltipState>(emptyTooltipState());
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
		interactionActive.value = false;
		tooltip.value = { ...tooltip.value, visible: false };
		await nextTick();
		if (!host.value || currentGeneration !== generation) return;

		try {
			const compiled = compileTrustedSpec(spec.value, host.value);
			const view = createTrustedView(
				compiled,
				host.value,
				handleTooltip,
				(active) => {
					interactionActive.value = active;
				},
			);
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

	async function zoomBy(factor: number): Promise<void> {
		if (activeView.value) await zoomView(activeView.value, factor);
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
		interactionActive: readonly(interactionActive),
		tooltip: readonly(tooltip),
		rerender: render,
		resetInteraction,
		zoomBy,
	};
}
