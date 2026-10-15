import { describe, expect, it } from "vitest";
import { inline, parseNotes } from "./markdown";

describe("inline", () => {
  it("reads code, bold and https links", () => {
    expect(inline("Run `falog-mcp` **now**: [guide](https://example.com/a) or https://example.com/b")).toEqual([
      { kind: "text", text: "Run " },
      { kind: "code", text: "falog-mcp" },
      { kind: "text", text: " " },
      { kind: "strong", text: "now" },
      { kind: "text", text: ": " },
      { kind: "link", text: "guide", href: "https://example.com/a" },
      { kind: "text", text: " or " },
      { kind: "link", text: "https://example.com/b", href: "https://example.com/b" },
    ]);
  });

  it("keeps links to anything but https as their text", () => {
    expect(inline("[plain](http://example.com) and [click](javascript:void)")).toEqual([
      { kind: "text", text: "plain" },
      { kind: "text", text: " and " },
      { kind: "text", text: "click" },
    ]);
  });

  it("leaves HTML as literal text", () => {
    expect(inline('<img src=x onerror="alert(1)">')).toEqual([
      { kind: "text", text: '<img src=x onerror="alert(1)">' },
    ]);
  });

  it("gives nothing for an empty line", () => {
    expect(inline("")).toEqual([]);
  });
});

describe("parseNotes", () => {
  it("reads release notes into headings, lists and paragraphs", () => {
    const notes = [
      "<!-- Release notes generated from the commits -->",
      "## Features",
      "- repeat tasks daily",
      "* speak Brazilian Portuguese",
      "",
      "A summary that",
      "goes on.",
      "",
      "### Fixes",
      "+ one fix",
    ].join("\r\n");
    expect(parseNotes(notes)).toEqual([
      { kind: "heading", spans: [{ kind: "text", text: "Features" }] },
      {
        kind: "list",
        items: [[{ kind: "text", text: "repeat tasks daily" }], [{ kind: "text", text: "speak Brazilian Portuguese" }]],
      },
      {
        kind: "paragraph",
        spans: [
          { kind: "text", text: "A summary that" },
          { kind: "text", text: " " },
          { kind: "text", text: "goes on." },
        ],
      },
      { kind: "heading", spans: [{ kind: "text", text: "Fixes" }] },
      { kind: "list", items: [[{ kind: "text", text: "one fix" }]] },
    ]);
  });

  it("starts a new list after anything else", () => {
    const blocks = parseNotes("- a\nbetween\n- b");
    expect(blocks.map((block) => block.kind)).toEqual(["list", "paragraph", "list"]);
  });

  it("gives no blocks for empty notes", () => {
    expect(parseNotes("")).toEqual([]);
    expect(parseNotes("\n\n<!-- only a comment -->\n")).toEqual([]);
  });
});
