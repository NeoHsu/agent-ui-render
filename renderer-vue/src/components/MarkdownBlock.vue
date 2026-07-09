<script lang="ts">
import { defineComponent, h, type VNodeChild } from "vue";
import {
  parseSafeMarkdown,
  type MarkdownBlockNode,
  type MarkdownInlineNode,
} from "../markdown";

export default defineComponent({
  name: "MarkdownBlock",
  props: {
    title: {
      type: String,
      required: false,
    },
    content: {
      type: String,
      required: true,
    },
  },
  setup(props) {
    return () =>
      h("section", { class: "card markdown-card" }, [
        props.title ? h("h2", props.title) : null,
        h(
          "div",
          { class: "report-prose" },
          parseSafeMarkdown(props.content).map((block, index) =>
            renderBlock(block, index),
          ),
        ),
      ]);
  },
});

function renderBlock(block: MarkdownBlockNode, index: number): VNodeChild {
  switch (block.type) {
    case "heading":
      return h(`h${block.level + 2}`, { key: index }, renderInline(block.children));
    case "paragraph":
      return h("p", { key: index }, renderInline(block.children));
    case "list":
      return h(
        block.ordered ? "ol" : "ul",
        { key: index },
        block.items.map((item, itemIndex) =>
          h("li", { key: itemIndex }, renderInline(item)),
        ),
      );
    case "blockquote":
      return h("blockquote", { key: index }, [
        h("p", renderInline(block.children)),
      ]);
    case "code":
      return h("pre", { key: index, "data-language": block.language }, [
        h("code", block.text),
      ]);
    case "hr":
      return h("hr", { key: index });
    default:
      return null;
  }
}

function renderInline(nodes: MarkdownInlineNode[]): VNodeChild[] {
  return nodes.map((node, index) => {
    switch (node.type) {
      case "text":
        return node.text;
      case "strong":
        return h("strong", { key: index }, renderInline(node.children));
      case "em":
        return h("em", { key: index }, renderInline(node.children));
      case "code":
        return h("code", { key: index }, node.text);
      case "semantic":
        return h(
          "span",
          { key: index, class: ["semantic", `semantic-${node.tone}`] },
          renderInline(node.children),
        );
      case "link": {
        const external = /^https?:\/\//i.test(node.href);
        return h(
          "a",
          {
            key: index,
            href: node.href,
            target: external ? "_blank" : undefined,
            rel: external ? "noreferrer" : undefined,
          },
          renderInline(node.children),
        );
      }
      default:
        return "";
    }
  });
}
</script>
