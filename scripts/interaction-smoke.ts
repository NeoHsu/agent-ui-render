import { promises as fs } from "node:fs";
import { join } from "node:path";

const port = Number(process.argv[2]);
const screenshotDirectory = process.argv[3];
if (!Number.isInteger(port) || port <= 0) {
	throw new Error(
		"Usage: bun scripts/interaction-smoke.ts <remote-debugging-port> [screenshot-directory]",
	);
}

type CdpResult = {
	result?: { result?: { value?: unknown }; data?: string };
	exceptionDetails?: unknown;
	error?: { message: string };
};

type Pending = {
	resolve: (value: CdpResult) => void;
	reject: (error: Error) => void;
};

const targets = (await fetch(`http://127.0.0.1:${port}/json`).then((response) =>
	response.json(),
)) as Array<{ type: string; url: string; webSocketDebuggerUrl: string }>;
const page = targets.find(
	(target) => target.type === "page" && target.url.startsWith("file:"),
);
if (!page) throw new Error("Chrome page target not found");

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise<void>((resolve, reject) => {
	socket.addEventListener("open", () => resolve(), { once: true });
	socket.addEventListener(
		"error",
		() => reject(new Error("CDP WebSocket failed")),
		{ once: true },
	);
});

let nextId = 1;
const pending = new Map<number, Pending>();
const externalRequests: string[] = [];
socket.addEventListener("message", (event) => {
	let message: CdpResult & {
		id?: number;
		method?: string;
		params?: { request?: { url?: unknown } };
	};
	try {
		message = JSON.parse(String(event.data)) as typeof message;
	} catch {
		return;
	}
	if (message.method === "Network.requestWillBeSent") {
		const url = message.params?.request?.url;
		if (typeof url === "string" && /^https?:/i.test(url)) {
			externalRequests.push(url);
		}
	}
	if (!message.id) return;
	const callback = pending.get(message.id);
	if (!callback) return;
	pending.delete(message.id);
	if (message.error) callback.reject(new Error(message.error.message));
	else callback.resolve(message);
});

function send(
	method: string,
	params: Record<string, unknown> = {},
): Promise<CdpResult> {
	const id = nextId++;
	socket.send(JSON.stringify({ id, method, params }));
	return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}

async function evaluate<T>(expression: string): Promise<T> {
	const response = await send("Runtime.evaluate", {
		expression,
		returnByValue: true,
	});
	if (response.exceptionDetails) {
		throw new Error(
			`Browser evaluation failed: ${JSON.stringify(response.exceptionDetails)}`,
		);
	}
	return response.result?.result?.value as T;
}

function assert(condition: unknown, message: string): asserts condition {
	if (!condition) throw new Error(message);
}

async function wait(milliseconds: number): Promise<void> {
	await new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function captureScreenshot(name: string): Promise<void> {
	if (!screenshotDirectory) return;
	const response = await send("Page.captureScreenshot", {
		format: "png",
		fromSurface: true,
		captureBeyondViewport: false,
	});
	const data = response.result?.data;
	assert(data, `Chrome did not return screenshot data for ${name}`);
	await fs.mkdir(screenshotDirectory, { recursive: true });
	const bytes = Uint8Array.from(atob(data), (character) =>
		character.charCodeAt(0),
	);
	await fs.writeFile(join(screenshotDirectory, `${name}.png`), bytes);
}

await send("Page.enable");
await send("Network.enable");
await wait(12_000);

const initial = await evaluate<{
	cards: number;
	svgs: number;
	errors: number;
	toolbars: number;
	resetButtons: number;
	zoomButtons: number;
	markTabStops: number;
	nestedTitles: number;
	technicalLegendLabels: number;
	trailXAxisLabels: number;
	chartShellBorder: number;
	interactionPosition: string;
	toolbarLaneHeightRange: number;
	interactionLegendOverlaps: number;
}>(`(() => ({
	cards: document.querySelectorAll('[data-view-intent="chart"]').length,
	svgs: document.querySelectorAll('.vega-chart svg').length,
	errors: document.querySelectorAll('.chart-render-error').length,
	toolbars: document.querySelectorAll('.chart-interaction-bar').length,
	resetButtons: document.querySelectorAll('.chart-reset-button').length,
	zoomButtons: document.querySelectorAll('.chart-icon-button').length,
	markTabStops: document.querySelectorAll('.vega-chart g.role-mark > [role="graphics-symbol"][tabindex="0"]').length,
	nestedTitles: [...document.querySelectorAll('.view-card')].filter((card) => {
		const title = card.querySelector(':scope > h2')?.textContent?.trim();
		return [...card.querySelectorAll('.vega-chart .role-title-text')].some((item) => item.textContent?.trim() === title);
	}).length,
	technicalLegendLabels: [...document.querySelectorAll('g.role-legend-label text')].filter((item) => item.textContent?.trim() === 'value_b').length,
	trailXAxisLabels: (() => {
		const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Trail');
		const axis = [...card.querySelectorAll('g.role-axis')].find((item) => item.getAttribute('aria-label')?.startsWith('X-axis'));
		return axis?.querySelectorAll('text').length ?? -1;
	})(),
	chartShellBorder: parseFloat(getComputedStyle(document.querySelector('.vega-chart-shell')).borderTopWidth),
	interactionPosition: getComputedStyle(document.querySelector('.chart-interaction-bar')).position,
	toolbarLaneHeightRange: (() => {
		const heights = [...document.querySelectorAll('.view-card-half .chart-toolbar-lane')].map((item) => item.getBoundingClientRect().height);
		return Math.max(...heights) - Math.min(...heights);
	})(),
	interactionLegendOverlaps: [...document.querySelectorAll('.chart-interaction-bar')].filter((bar) => {
		const card = bar.closest('.view-card');
		const legend = card?.querySelector('g.role-legend');
		if (!legend) return false;
		const a = bar.getBoundingClientRect();
		const b = legend.getBoundingClientRect();
		return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
	}).length
}))()`);
assert(initial.cards === 44, `expected 44 chart cards, got ${initial.cards}`);
assert(initial.svgs === 44, `expected 44 SVG charts, got ${initial.svgs}`);
assert(initial.errors === 0, `expected no chart errors, got ${initial.errors}`);
assert(
	initial.toolbars === 5,
	`expected 5 interaction toolbars, got ${initial.toolbars}`,
);
assert(
	initial.resetButtons === 0,
	`expected reset controls to stay hidden until active, got ${initial.resetButtons}`,
);
assert(
	initial.zoomButtons === 2,
	"zoom charts should expose minus and plus controls",
);
assert(
	initial.markTabStops === 44,
	"each chart should expose one roving mark tab stop",
);
assert(
	initial.nestedTitles === 0,
	"card titles should not be repeated inside plots",
);
assert(
	initial.technicalLegendLabels === 0,
	"folded series legends should use presentation labels",
);
assert(
	initial.trailXAxisLabels > 0 && initial.trailXAxisLabels <= 8,
	`Trail x-axis should stay readable, got ${initial.trailXAxisLabels} labels`,
);
assert(
	initial.chartShellBorder === 0,
	"plot shell should not create a nested card frame",
);
assert(
	initial.interactionPosition === "static",
	"interaction controls should stay in the dedicated toolbar lane",
);
assert(
	initial.toolbarLaneHeightRange === 0,
	`half-width charts should share toolbar height, range ${initial.toolbarLaneHeightRange}`,
);
assert(
	initial.interactionLegendOverlaps === 0,
	"interaction controls should not overlap chart legends",
);

const tooltipPoint = await evaluate<{ x: number; y: number }>(`(() => {
	const mark = document.querySelector('.vega-chart g.mark-symbol.role-mark > [role="graphics-symbol"]');
	const rect = mark.getBoundingClientRect();
	window.scrollTo(0, Math.max(0, rect.top + window.scrollY - 260));
	const next = mark.getBoundingClientRect();
	return {x: next.left + next.width / 2, y: next.top + next.height / 2};
})()`);
await send("Input.dispatchMouseEvent", {
	type: "mouseMoved",
	x: tooltipPoint.x,
	y: tooltipPoint.y,
});
await wait(400);
const tooltipState = await evaluate<{
	text: string;
	header: boolean;
	swatch: boolean;
}>(`(() => {
	const tooltip = document.querySelector('.chart-tooltip');
	return {
		text: tooltip?.textContent?.trim() || '',
		header: Boolean(tooltip?.querySelector('.chart-tooltip-header')),
		swatch: Boolean(tooltip?.querySelector('.chart-tooltip-swatch'))
	};
})()`);
assert(
	tooltipState.text.includes("Value") &&
		tooltipState.header &&
		tooltipState.swatch,
	"pointer hover should display a structured tooltip with a series swatch",
);
await captureScreenshot("01-tooltip");

await evaluate<void>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Grouped Bar');
	card.querySelector('g.role-mark > [role="graphics-symbol"]').focus();
})()`);
await send("Input.dispatchKeyEvent", {
	type: "keyDown",
	key: "Enter",
	code: "Enter",
});
await send("Input.dispatchKeyEvent", {
	type: "keyUp",
	key: "Enter",
	code: "Enter",
});
await wait(400);
const selectedBars = await evaluate<{
	dimmed: number;
	active: boolean;
}>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Grouped Bar');
	return {
		dimmed: [...card.querySelectorAll('g.role-mark > [role="graphics-symbol"]')].filter((item) => item.getAttribute('opacity') === '0.4').length,
		active: card.querySelector('.chart-interaction-bar')?.dataset.active === 'true' && !card.querySelector('.chart-reset-button')?.disabled
	};
})()`);
assert(
	selectedBars.dimmed > 0 && selectedBars.active,
	"keyboard Enter should activate selection controls and dim peers",
);
await captureScreenshot("02-click-selection");
await evaluate<void>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Grouped Bar');
	card.querySelector('.chart-reset-button').click();
})()`);
await wait(500);
const resetBars = await evaluate<{ dimmed: number; active: boolean }>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Grouped Bar');
	return {
		dimmed: [...card.querySelectorAll('g.role-mark > [role="graphics-symbol"]')].filter((item) => item.getAttribute('opacity') === '0.4').length,
		active: card.querySelector('.chart-interaction-bar')?.dataset.active === 'true'
	};
})()`);
assert(
	resetBars.dimmed === 0 && !resetBars.active,
	"Reset should clear click selection and control state",
);

await evaluate<void>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Density');
	card.querySelector('g.role-legend-symbol > [role="graphics-symbol"]').focus();
})()`);
await send("Input.dispatchKeyEvent", {
	type: "keyDown",
	key: "Enter",
	code: "Enter",
});
await send("Input.dispatchKeyEvent", {
	type: "keyUp",
	key: "Enter",
	code: "Enter",
});
await wait(400);
const legendDimmed = await evaluate<number>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Density');
	return [...card.querySelectorAll('g.role-mark > [role="graphics-symbol"]')].filter((item) => item.getAttribute('opacity') === '0.4').length;
})()`);
assert(legendDimmed > 0, "legend selection should dim unselected series");
await captureScreenshot("03-legend-selection");

const brush = await evaluate<{
	x1: number;
	y1: number;
	x2: number;
	y2: number;
}>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Brush Scatter');
	const plot = card.querySelector('g.mark-symbol.role-mark');
	const rect = plot.getBoundingClientRect();
	window.scrollTo(0, Math.max(0, rect.top + window.scrollY - 260));
	const next = plot.getBoundingClientRect();
	return {x1: next.left + 2, y1: next.top + 2, x2: next.left + next.width * 0.55, y2: next.top + next.height * 0.55};
})()`);
await send("Input.dispatchMouseEvent", {
	type: "mouseMoved",
	x: brush.x1,
	y: brush.y1,
});
await send("Input.dispatchMouseEvent", {
	type: "mousePressed",
	x: brush.x1,
	y: brush.y1,
	button: "left",
	buttons: 1,
	clickCount: 1,
});
await send("Input.dispatchMouseEvent", {
	type: "mouseMoved",
	x: brush.x2,
	y: brush.y2,
	button: "left",
	buttons: 1,
});
await send("Input.dispatchMouseEvent", {
	type: "mouseReleased",
	x: brush.x2,
	y: brush.y2,
	button: "left",
	buttons: 0,
	clickCount: 1,
});
await wait(400);
const brushState = await evaluate<{ dimmed: number; total: number }>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Brush Scatter');
	const marks = [...card.querySelectorAll('g.role-mark > [role="graphics-symbol"]')];
	return {dimmed: marks.filter((item) => item.getAttribute('opacity') === '0.4').length, total: marks.length};
})()`);
assert(
	brushState.dimmed > 0 && brushState.dimmed < brushState.total,
	"brush should retain selected marks and dim peers",
);
await captureScreenshot("04-brush-selection");

const zoom = await evaluate<{
	x1: number;
	y1: number;
	x2: number;
	y2: number;
}>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	const plot = card.querySelector('g.mark-line.role-mark');
	const rect = plot.getBoundingClientRect();
	window.scrollTo(0, Math.max(0, rect.top + window.scrollY - 260));
	const next = plot.getBoundingClientRect();
	return {x1: next.left + next.width * 0.15, y1: next.top + next.height * 0.15, x2: next.left + next.width * 0.75, y2: next.top + next.height * 0.75};
})()`);
await send("Input.dispatchMouseEvent", {
	type: "mouseMoved",
	x: zoom.x1,
	y: zoom.y1,
});
await send("Input.dispatchMouseEvent", {
	type: "mousePressed",
	x: zoom.x1,
	y: zoom.y1,
	button: "left",
	buttons: 1,
	clickCount: 1,
});
await send("Input.dispatchMouseEvent", {
	type: "mouseMoved",
	x: zoom.x2,
	y: zoom.y2,
	button: "left",
	buttons: 1,
});
await send("Input.dispatchMouseEvent", {
	type: "mouseReleased",
	x: zoom.x2,
	y: zoom.y2,
	button: "left",
	buttons: 0,
	clickCount: 1,
});
await wait(400);
const zoomBrush = await evaluate<string>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	return card.querySelector('g.agent_zoom_brush > path').getAttribute('d');
})()`);
assert(
	zoomBrush !== "M0,0h0v0h0Z",
	"zoom drag should create a bound scale interval",
);
await captureScreenshot("05-zoom");
await evaluate<void>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	card.querySelector('.chart-reset-button').click();
})()`);
await wait(600);
const resetZoomBrush = await evaluate<string>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	return card.querySelector('g.agent_zoom_brush > path').getAttribute('d');
})()`);
assert(
	resetZoomBrush === "M0,0h0v0h0Z",
	"Reset should clear the zoom interval",
);
await evaluate<void>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	card.querySelector('.chart-icon-button[aria-label="Zoom in"]').click();
})()`);
await wait(500);
const programmaticZoomResult = await evaluate<{
	brush: string;
	active: boolean;
}>(`(() => {
	const card = [...document.querySelectorAll('.view-card')].find((item) => item.querySelector('h2')?.textContent.trim() === 'Zoomable Line');
	return {
		brush: card.querySelector('g.agent_zoom_brush > path').getAttribute('d'),
		active: card.querySelector('.chart-interaction-bar')?.dataset.active === 'true'
	};
})()`);
assert(
	programmaticZoomResult.brush !== "M0,0h0v0h0Z" &&
		programmaticZoomResult.active,
	"zoom controls should create an interval and activate reset state",
);
await captureScreenshot("06-zoom-controls");

assert(
	externalRequests.length === 0,
	`rich HTML attempted external network requests: ${externalRequests.join(", ")}`,
);
process.stdout.write(
	"interaction smoke OK: no-network, tooltip, keyboard, click, reset, legend, brush, zoom\n",
);
socket.close();
