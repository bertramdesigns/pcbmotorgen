import { describe, expect, it } from "vitest";
import { markdownToHtml, renderInline } from "./markdown";

describe("renderInline", () => {
  it("escapes HTML-significant characters", () => {
    // Quotes are safe unescaped in text content (they ARE escaped in attrs).
    expect(renderInline('a <b> & "c"')).toBe('a &lt;b&gt; &amp; "c"');
  });

  it("renders code spans with their contents escaped and unformatted", () => {
    expect(renderInline("use `Vec<PatternParameter>` **not**")).toBe(
      "use <code>Vec&lt;PatternParameter&gt;</code> <strong>not</strong>",
    );
  });

  it("renders bold, star-italic and underscore-italic", () => {
    expect(renderInline("**bold** and *em* and _em2_")).toBe(
      "<strong>bold</strong> and <em>em</em> and <em>em2</em>",
    );
  });

  it("never mangles snake_case outside code spans", () => {
    expect(renderInline("the y_min_mm and y_max_mm bounds")).toBe(
      "the y_min_mm and y_max_mm bounds",
    );
  });

  it("renders http(s) links as real anchors", () => {
    expect(renderInline("see [docs](https://example.com/a) now")).toBe(
      'see <a href="https://example.com/a" target="_blank" rel="noopener noreferrer">docs</a> now',
    );
  });

  it("renders relative repo links as inert code refs", () => {
    expect(renderInline("see [`handoff.md`](./routing-pattern-handoff.md)")).toBe(
      'see <code class="md-link-ref">handoff.md</code>',
    );
  });

  it("does not treat javascript: hrefs as links", () => {
    // The href token stops at the first `)`, so `alert(1` is inert and only
    // the trailing `)` survives as text — either way nothing executes.
    expect(renderInline("[x](javascript:alert(1))")).toBe(
      '<code class="md-link-ref">x</code>)',
    );
  });
});

describe("markdownToHtml — blocks", () => {
  it("renders ATX headings at all levels", () => {
    expect(markdownToHtml("# One\n## Two\n###### Six")).toBe(
      "<h1>One</h1>\n<h2>Two</h2>\n<h6>Six</h6>",
    );
  });

  it("renders fenced code blocks with a language class and escaped body", () => {
    expect(markdownToHtml("```rust\nfn f<T>() -> u32 { 0 }\n```")).toBe(
      '<pre><code class="language-rust">fn f&lt;T&gt;() -&gt; u32 { 0 }</code></pre>',
    );
  });

  it("renders fenced code blocks without a language", () => {
    expect(markdownToHtml("```\nplain <text>\n```")).toBe(
      "<pre><code>plain &lt;text&gt;</code></pre>",
    );
  });

  it("renders thematic breaks", () => {
    expect(markdownToHtml("a\n\n---\n\nb")).toBe("<p>a</p>\n<hr>\n<p>b</p>");
  });

  it("renders paragraphs, collapsing wrapped lines", () => {
    expect(markdownToHtml("first line\nsecond line\n\nnext")).toBe(
      "<p>first line second line</p>\n<p>next</p>",
    );
  });

  it("renders unordered lists with lazy continuation", () => {
    expect(markdownToHtml("- one\n  wrapped\n- two")).toBe(
      "<ul><li>one wrapped</li><li>two</li></ul>",
    );
  });

  it("renders ordered lists with continuation lines", () => {
    expect(markdownToHtml("1. Choose\n   continued\n2. Load")).toBe(
      "<ol><li>Choose continued</li><li>Load</li></ol>",
    );
  });

  it("renders task-list items (unchecked and checked)", () => {
    expect(markdownToHtml("- [ ] todo\n- [x] done")).toBe(
      '<ul><li><input type="checkbox" disabled> todo</li>' +
        '<li><input type="checkbox" disabled checked> done</li></ul>',
    );
  });

  it("renders tables with alignment and inline formatting", () => {
    const md = [
      "| field | type | meaning |",
      "| --- | :---: | ---: |",
      "| `key` | **String** | the `x` |",
    ].join("\n");
    expect(markdownToHtml(md)).toBe(
      "<table><thead><tr><th>field</th><th style=\"text-align: center;\">type</th>" +
        '<th style="text-align: right;">meaning</th></tr></thead>' +
        "<tbody><tr><td><code>key</code></td><td style=\"text-align: center;\"><strong>String</strong></td>" +
        '<td style="text-align: right;">the <code>x</code></td></tr></tbody></table>',
    );
  });

  it("handles escaped pipes inside table cells", () => {
    const md = ['| type | meaning |', '| --- | --- |', '| `"int" \\| "float"` | kinds |'].join("\n");
    expect(markdownToHtml(md)).toContain('<td><code>"int" | "float"</code></td>');
  });

  it("renders blockquotes by recursing into their content", () => {
    expect(markdownToHtml("> **Warning:** careful\n> with this")).toBe(
      "<blockquote><p><strong>Warning:</strong> careful with this</p></blockquote>",
    );
  });

  it("never passes raw HTML through", () => {
    const html = markdownToHtml('<img src=x onerror=alert(1)> and <script>x</script>');
    expect(html).not.toContain("<img");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;img");
  });

  it("keeps a list from swallowing a following fence", () => {
    const md = "- item\n\n```python\nx = 1\n```";
    expect(markdownToHtml(md)).toBe(
      '<ul><li>item</li></ul>\n<pre><code class="language-python">x = 1</code></pre>',
    );
  });
});
