import { describe, expect, it } from "vitest";
import { markdownToHtml, parseSafeMarkdown } from "../src/markdown.js";

describe("safe markdown", () => {
	it("parses the governed block subset", () => {
		const blocks = parseSafeMarkdown(
			"## Summary\n\n- one\n- two\n\n> source faithful",
		);
		expect(blocks.map((block) => block.type)).toEqual([
			"heading",
			"list",
			"blockquote",
		]);
	});

	it("escapes raw markup and fenced code", () => {
		const html = markdownToHtml(
			'<script>alert("unsafe")</script>\n\n```html\n<img src=x>\n```',
		);
		expect(html).not.toContain("<script>");
		expect(html).not.toContain("<img");
		expect(html).toContain("&lt;script&gt;");
		expect(html).toContain("&lt;img src=x&gt;");
	});

	it("allows safe links and semantic tones while rejecting script URLs", () => {
		const html = markdownToHtml(
			"[guide](https://example.com/docs) [unsafe](javascript:alert(1)) {warning: verify this}",
		);
		expect(html).toContain(
			'<a href="https://example.com/docs" target="_blank" rel="noreferrer">guide</a>',
		);
		expect(html).toContain('class="semantic semantic-warning"');
		expect(html).not.toContain('href="javascript:');
	});
});
