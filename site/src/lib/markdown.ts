// A small markdown subset for release notes, parsed into plain data that the page renders as text:
// headings, bullet lists and paragraphs; inside them `code`, **bold**, [text](https://...) and bare
// https links. Anything else, HTML included, stays literal text.

export type Span =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "strong"; text: string }
  | { kind: "link"; text: string; href: string };

export type Block =
  | { kind: "heading"; spans: Span[] }
  | { kind: "paragraph"; spans: Span[] }
  | { kind: "list"; items: Span[][] };

const INLINE = /`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\(([^)\s]+)\)|(https:\/\/[^\s)<>]+)/g;

function https(url: string): string | undefined {
  return /^https:\/\/[^\s"<>]+$/.test(url) ? url : undefined;
}

export function inline(text: string): Span[] {
  const spans: Span[] = [];
  let last = 0;
  for (const match of text.matchAll(INLINE)) {
    const index = match.index ?? 0;
    if (index > last) spans.push({ kind: "text", text: text.slice(last, index) });
    if (match[1] != null) spans.push({ kind: "code", text: match[1] });
    else if (match[2] != null) spans.push({ kind: "strong", text: match[2] });
    else if (match[3] != null) {
      const href = https(match[4]);
      spans.push(href ? { kind: "link", text: match[3], href } : { kind: "text", text: match[3] });
    } else spans.push({ kind: "link", text: match[5], href: match[5] });
    last = index + match[0].length;
  }
  if (last < text.length) spans.push({ kind: "text", text: text.slice(last) });
  return spans;
}

export function parseNotes(source: string): Block[] {
  const blocks: Block[] = [];
  let list: { kind: "list"; items: Span[][] } | null = null;
  let paragraph: { kind: "paragraph"; spans: Span[] } | null = null;
  for (const raw of source.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trim();
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    const item = /^[-*+]\s+(.*)$/.exec(line);
    if (!line || heading || item) paragraph = null;
    if (!item) list = null;
    if (!line || /^<!--.*-->$/.test(line)) continue;
    if (heading) {
      blocks.push({ kind: "heading", spans: inline(heading[1]) });
    } else if (item) {
      if (!list) blocks.push((list = { kind: "list", items: [] }));
      list.items.push(inline(item[1]));
    } else if (paragraph) {
      paragraph.spans.push({ kind: "text", text: " " }, ...inline(line));
    } else {
      blocks.push((paragraph = { kind: "paragraph", spans: inline(line) }));
    }
  }
  return blocks;
}
