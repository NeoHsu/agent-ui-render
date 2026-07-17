import { isSemanticTone, type SemanticTone } from "./format.js";

export type MarkdownInlineNode =
	| { type: "text"; text: string }
	| { type: "strong"; children: MarkdownInlineNode[] }
	| { type: "em"; children: MarkdownInlineNode[] }
	| { type: "code"; text: string }
	| { type: "semantic"; tone: SemanticTone; children: MarkdownInlineNode[] }
	| { type: "link"; href: string; children: MarkdownInlineNode[] };

export type MarkdownBlockNode =
	| { type: "heading"; level: 1 | 2 | 3; children: MarkdownInlineNode[] }
	| { type: "paragraph"; children: MarkdownInlineNode[] }
	| { type: "list"; ordered: boolean; items: MarkdownInlineNode[][] }
	| { type: "blockquote"; children: MarkdownInlineNode[] }
	| { type: "code"; language?: string; text: string }
	| { type: "hr" };

export function parseSafeMarkdown(source: string): MarkdownBlockNode[] {
	const lines = source.replace(/\r\n?/g, "\n").trimEnd().split("\n");
	const blocks: MarkdownBlockNode[] = [];
	let index = 0;

	while (index < lines.length) {
		const line = lines[index] ?? "";
		if (!line.trim()) {
			index += 1;
			continue;
		}

		const fence = line.match(/^```([A-Za-z0-9_-]+)?\s*$/);
		if (fence) {
			const language = fence[1];
			index += 1;
			const codeLines: string[] = [];
			while (index < lines.length && !/^```\s*$/.test(lines[index] ?? "")) {
				codeLines.push(lines[index] ?? "");
				index += 1;
			}
			if (index < lines.length) index += 1;
			const block: MarkdownBlockNode = {
				type: "code",
				text: codeLines.join("\n"),
			};
			if (language) block.language = language;
			blocks.push(block);
			continue;
		}

		if (/^\s*(?:---|\*\*\*)\s*$/.test(line)) {
			blocks.push({ type: "hr" });
			index += 1;
			continue;
		}

		const heading = line.match(/^(#{1,3})\s+(.+)$/);
		if (heading?.[1] && heading[2]) {
			blocks.push({
				type: "heading",
				level: heading[1].length as 1 | 2 | 3,
				children: parseInlineMarkdown(heading[2].trim()),
			});
			index += 1;
			continue;
		}

		if (/^\s*>\s?/.test(line)) {
			const quoteLines: string[] = [];
			while (index < lines.length && /^\s*>\s?/.test(lines[index] ?? "")) {
				quoteLines.push((lines[index] ?? "").replace(/^\s*>\s?/, ""));
				index += 1;
			}
			blocks.push({
				type: "blockquote",
				children: parseInlineMarkdown(quoteLines.join(" ").trim()),
			});
			continue;
		}

		const listMatch = line.match(/^\s*(?:[-*]|\d+\.)\s+(.+)$/);
		if (listMatch) {
			const ordered = /^\s*\d+\./.test(line);
			const items: MarkdownInlineNode[][] = [];
			while (index < lines.length) {
				const candidate = lines[index] ?? "";
				const match = candidate.match(
					ordered ? /^\s*\d+\.\s+(.+)$/ : /^\s*[-*]\s+(.+)$/,
				);
				if (!match?.[1]) break;
				items.push(parseInlineMarkdown(match[1].trim()));
				index += 1;
			}
			blocks.push({ type: "list", ordered, items });
			continue;
		}

		const paragraphLines: string[] = [];
		while (index < lines.length) {
			const candidate = lines[index] ?? "";
			if (!candidate.trim()) break;
			if (paragraphLines.length > 0 && isBlockStart(candidate)) break;
			paragraphLines.push(candidate.trim());
			index += 1;
		}
		blocks.push({
			type: "paragraph",
			children: parseInlineMarkdown(paragraphLines.join(" ")),
		});
	}

	return blocks;
}

export function parseInlineMarkdown(
	value: string,
	depth = 0,
): MarkdownInlineNode[] {
	if (!value) return [];
	if (depth > 6) return [{ type: "text", text: value }];

	const nodes: MarkdownInlineNode[] = [];
	let buffer = "";
	let index = 0;

	function flush(): void {
		if (!buffer) return;
		nodes.push({ type: "text", text: buffer });
		buffer = "";
	}

	while (index < value.length) {
		if (value[index] === "`") {
			const end = value.indexOf("`", index + 1);
			if (end > index + 1) {
				flush();
				nodes.push({ type: "code", text: value.slice(index + 1, end) });
				index = end + 1;
				continue;
			}
		}

		if (value[index] === "{") {
			const match = value.slice(index).match(/^\{([a-z]+):\s*/i);
			const tone = match?.[1]?.toLowerCase();
			if (tone && isSemanticTone(tone)) {
				const contentStart = index + (match?.[0].length ?? 0);
				const end = value.indexOf("}", contentStart);
				if (end > contentStart) {
					flush();
					nodes.push({
						type: "semantic",
						tone,
						children: parseInlineMarkdown(
							value.slice(contentStart, end),
							depth + 1,
						),
					});
					index = end + 1;
					continue;
				}
			}
		}

		if (value.startsWith("**", index)) {
			const end = value.indexOf("**", index + 2);
			if (end > index + 2) {
				flush();
				nodes.push({
					type: "strong",
					children: parseInlineMarkdown(value.slice(index + 2, end), depth + 1),
				});
				index = end + 2;
				continue;
			}
		}

		if (value[index] === "*" && !value.startsWith("**", index)) {
			const end = value.indexOf("*", index + 1);
			if (end > index + 1) {
				flush();
				nodes.push({
					type: "em",
					children: parseInlineMarkdown(value.slice(index + 1, end), depth + 1),
				});
				index = end + 1;
				continue;
			}
		}

		if (value[index] === "[") {
			const closeLabel = value.indexOf("](", index + 1);
			const closeHref =
				closeLabel >= 0 ? value.indexOf(")", closeLabel + 2) : -1;
			if (closeLabel > index + 1 && closeHref > closeLabel + 2) {
				const label = value.slice(index + 1, closeLabel);
				const href = sanitizeHref(value.slice(closeLabel + 2, closeHref));
				flush();
				if (href) {
					nodes.push({
						type: "link",
						href,
						children: parseInlineMarkdown(label, depth + 1),
					});
				} else {
					nodes.push(...parseInlineMarkdown(label, depth + 1));
				}
				index = closeHref + 1;
				continue;
			}
		}

		buffer += value[index] ?? "";
		index += 1;
	}

	flush();
	return nodes;
}

export function markdownToHtml(source: string): string {
	return parseSafeMarkdown(source).map(renderBlockHtml).join("\n");
}

function renderBlockHtml(block: MarkdownBlockNode): string {
	switch (block.type) {
		case "heading": {
			const tag = `h${block.level + 2}`;
			return `<${tag}>${renderInlineHtml(block.children)}</${tag}>`;
		}
		case "paragraph":
			return `<p>${renderInlineHtml(block.children)}</p>`;
		case "list": {
			const tag = block.ordered ? "ol" : "ul";
			const items = block.items
				.map((item) => `<li>${renderInlineHtml(item)}</li>`)
				.join("");
			return `<${tag}>${items}</${tag}>`;
		}
		case "blockquote":
			return `<blockquote><p>${renderInlineHtml(block.children)}</p></blockquote>`;
		case "code": {
			const language = block.language
				? ` data-language="${escapeHtml(block.language)}"`
				: "";
			return `<pre${language}><code>${escapeHtml(block.text)}</code></pre>`;
		}
		case "hr":
			return "<hr>";
		default:
			return "";
	}
}

function renderInlineHtml(nodes: MarkdownInlineNode[]): string {
	return nodes
		.map((node) => {
			switch (node.type) {
				case "text":
					return escapeHtml(node.text);
				case "strong":
					return `<strong>${renderInlineHtml(node.children)}</strong>`;
				case "em":
					return `<em>${renderInlineHtml(node.children)}</em>`;
				case "code":
					return `<code>${escapeHtml(node.text)}</code>`;
				case "semantic":
					return `<span class="semantic semantic-${node.tone}">${renderInlineHtml(node.children)}</span>`;
				case "link": {
					const external = /^https?:\/\//i.test(node.href);
					const attrs = external
						? ' target="_blank" rel="noopener noreferrer"'
						: "";
					return `<a href="${escapeHtml(node.href)}"${attrs}>${renderInlineHtml(node.children)}</a>`;
				}
				default:
					return "";
			}
		})
		.join("");
}

function isBlockStart(line: string): boolean {
	return (
		/^```/.test(line) ||
		/^(#{1,3})\s+/.test(line) ||
		/^\s*(?:---|\*\*\*)\s*$/.test(line) ||
		/^\s*>\s?/.test(line) ||
		/^\s*(?:[-*]|\d+\.)\s+/.test(line)
	);
}

function sanitizeHref(value: string): string {
	const href = value;
	if (!href || /[\u0000-\u001f\u007f\s]/.test(href)) return "";
	if (/^(?:https?:|mailto:)/i.test(href)) return href;
	if (href.startsWith("#")) return href;
	if (href.startsWith("/") && !href.startsWith("//")) return href;
	return "";
}

function escapeHtml(value: string): string {
	return value
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;")
		.replace(/'/g, "&#39;");
}
