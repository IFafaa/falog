import { describe, expect, it } from "vitest";
import en from "./en";
import pt from "./pt";
import { COPY, LANGS, asset, otherLang, pageUrl } from "./index";

/** Every text of a copy object by its path (`home.views.0.caption`), functions included. */
function texts(value: unknown, path = ""): Map<string, unknown> {
  const found = new Map<string, unknown>();
  if (typeof value === "string" || typeof value === "function") {
    found.set(path, value);
  } else if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      for (const [childPath, text] of texts(child, path ? `${path}.${key}` : key)) found.set(childPath, text);
    }
  }
  return found;
}

/** The `{name}` placeholders a text is filled in with. */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
}

// Roadmap entries are keyed by the spec's English text and fall back to it when missing, so they are
// not required in every language.
const { roadmap: _enRoadmap, ...enCopy } = en;
const { roadmap: _ptRoadmap, ...ptCopy } = pt;
const english = texts(enCopy);
const portuguese = texts(ptCopy);

describe("the Portuguese copy", () => {
  it("has an entry for every English text", () => {
    const missing = [...english.keys()].filter((path) => {
      const text = portuguese.get(path);
      return typeof text === "string" ? text.trim() === "" : typeof text !== "function";
    });
    expect(missing).toEqual([]);
  });

  it("has nothing the English copy lacks", () => {
    expect([...portuguese.keys()].filter((path) => !english.has(path))).toEqual([]);
  });

  it("keeps the placeholders the pages fill in", () => {
    for (const [path, text] of english) {
      if (typeof text !== "string") continue;
      expect(placeholders(portuguese.get(path) as string), path).toEqual(placeholders(text));
    }
  });

  it("counts in its own words", () => {
    expect(en.home.moreShipped(3)).toBe("and 3 more");
    expect(pt.home.moreShipped(3)).toBe("e mais 3");
  });
});

describe("pages and languages", () => {
  it("lists both languages with their copy", () => {
    expect(LANGS).toEqual(["en", "pt"]);
    expect(COPY.en.lang).toBe("en");
    expect(COPY.pt.htmlLang).toBe("pt-BR");
    expect(otherLang("en")).toBe("pt");
    expect(otherLang("pt")).toBe("en");
  });

  it("puts every page under the /falog/ base, English without a prefix", () => {
    expect(pageUrl("en", "home")).toBe("/falog/");
    expect(pageUrl("en", "download")).toBe("/falog/download/");
    expect(pageUrl("pt", "home")).toBe("/falog/pt/");
    expect(pageUrl("pt", "releases")).toBe("/falog/pt/releases/");
    expect(asset("img/board-dark.webp")).toBe("/falog/img/board-dark.webp");
  });
});
