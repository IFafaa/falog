// The site's languages and pages, and the URL of each page in each language.

import en, { type Copy } from "./en";
import pt from "./pt";

export type Lang = "en" | "pt";
export type Page = "home" | "download" | "releases";

export const LANGS: Lang[] = ["en", "pt"];
export const COPY: Record<Lang, Copy> = { en, pt };
export const REPO = "https://github.com/IFafaa/falog";

const SLUGS: Record<Page, string> = { home: "", download: "download/", releases: "releases/" };

/** `/falog/`, always with one trailing slash. */
const BASE = import.meta.env.BASE_URL.replace(/\/*$/, "/");

/** The path of `page` in `lang`: `/falog/download/`, `/falog/pt/download/`. */
export function pageUrl(lang: Lang, page: Page): string {
  return BASE + (lang === "en" ? "" : `${lang}/`) + SLUGS[page];
}

/** A file in `public/`, under the base path. */
export function asset(path: string): string {
  return BASE + path;
}

export function otherLang(lang: Lang): Lang {
  return lang === "en" ? "pt" : "en";
}
