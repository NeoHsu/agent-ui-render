import { promises as fs } from "node:fs";
import { gzipSync } from "node:zlib";

const DEFAULT_RAW_BUDGET_BYTES = 925_000;
const DEFAULT_GZIP_BUDGET_BYTES = 320_000;

function budgetFromEnvironment(name: string, fallback: number): number {
	const value = process.env[name];
	if (value === undefined) return fallback;
	const parsed = Number(value);
	if (!Number.isSafeInteger(parsed) || parsed <= 0) {
		throw new Error(`${name} must be a positive integer, got ${JSON.stringify(value)}`);
	}
	return parsed;
}

const rawBudget = budgetFromEnvironment(
	"RENDERER_JS_RAW_BUDGET_BYTES",
	DEFAULT_RAW_BUDGET_BYTES,
);
const gzipBudget = budgetFromEnvironment(
	"RENDERER_JS_GZIP_BUDGET_BYTES",
	DEFAULT_GZIP_BUDGET_BYTES,
);
const renderer = await fs.readFile(
	new URL("../generated/renderer.js", import.meta.url),
);
const rawBytes = renderer.byteLength;
const gzipBytes = gzipSync(renderer, { level: 9 }).byteLength;
const failures: string[] = [];
if (rawBytes > rawBudget) {
	failures.push(`raw ${rawBytes} bytes exceeds ${rawBudget}`);
}
if (gzipBytes > gzipBudget) {
	failures.push(`gzip ${gzipBytes} bytes exceeds ${gzipBudget}`);
}
if (failures.length > 0) {
	throw new Error(`renderer.js performance budget failed: ${failures.join("; ")}`);
}
process.stdout.write(
	`renderer.js budget OK: raw ${rawBytes}/${rawBudget} bytes, gzip ${gzipBytes}/${gzipBudget} bytes\n`,
);
