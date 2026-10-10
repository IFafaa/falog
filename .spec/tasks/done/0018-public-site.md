# 0018: Public site

**Status:** done
**Area:** docs, ci

## Goal

Anyone who hears about Falog can see what it is and everything it does, download the most stable build
for their system in one click, and follow what shipped and what comes next, in English or Brazilian
Portuguese.

## Context

Releases are published by a tag-triggered workflow (`release.yml`, separate work) with fixed,
version-free asset names on every release, so `releases/latest/download/<name>` always points at the
newest stable build. Stable means a GitHub release that is not a prerelease; prereleases (tags such as
`v0.3.0-beta.1`) are previews. macOS builds are not notarized yet and the Windows installer is not
code-signed. The site is hosted on GitHub Pages at <https://ifafaa.github.io/falog/>.

The site follows the app's look ([design-system.md](../../design-system.md#public-site)): dark by
default, dense, monospace accents, the One Dark and One Light tokens, IBM Plex Sans and Lilex, real
screenshots in both themes.

## Scope

- In: an Astro project in `site/` (static output) with three pages in two languages, screenshots of
  the app with demo data, `.github/workflows/pages.yml`, one README line.
- Out: a custom domain, analytics, a blog, more languages; the release workflow itself.

## Acceptance criteria

- [x] Pages `/`, `/download/`, `/releases/` in English and `/pt/`, `/pt/download/`, `/pt/releases/` in
      Brazilian Portuguese, under the `/falog` base; a shared header (page links, language switch,
      theme toggle) and footer; `hreflang` alternates and a canonical link on every page
- [x] All copy in `site/src/i18n/en.ts` and `pt.ts`, written for each language
- [x] First visit: when the browser's first language is the other one, a banner offers it; the choice
      or a dismissal is remembered, and the site never redirects
- [x] Home: hero with "Download for <your system>" (Apple silicon or Intel on a Mac when the browser
      tells), the five views with screenshots, eight features (assistant, local voice, agents with equal
      weight for Claude Code, Gemini CLI, Codex and any ACP agent, areas, calendar, local and private,
      three systems, open source), the latest release and the roadmap
- [x] Calendar described as Falog's own, with Google Calendar as today's provider (iCal link or
      sign-in) and more providers to come; nothing claimed for providers that are not tested
- [x] Download: cards for Windows, macOS and Linux with the fixed asset URLs, the Linux one-liner with a
      copy button, the SmartScreen and Open Anyway notes, and the newest preview when it is newer than
      the stable release, marked less stable
- [x] Releases: every release with version, date, stable/preview badge and notes rendered as text (a
      small markdown subset built with DOM nodes; no HTML from GitHub is ever injected)
- [x] No release yet, GitHub unreachable or rate limited, or an asset missing from the latest
      release: the pages still work and never link to a 404 they know about
- [x] Light and dark themes, responsive down to 360 px, keyboard-accessible tabs, visible focus, alt
      text on every screenshot, text contrast at least 4.5:1
- [x] `pages.yml` builds on pull requests that touch the site and deploys on pushes to `main`, with
      actions pinned to SHAs and minimal permissions
- [x] `scripts/check.ps1 -NoVoice` passes (policy and typos cover `site/`)

## Plan

1. Seed demo data, run the debug build against it with a temporary local hook that picks the view,
   theme and zoom, capture with PrintWindow at 1.5× and convert to WebP.
2. Astro project: per-language copy, one layout, a component per page, thin routes per language.
3. Check every page in both languages at desktop and phone widths, in both themes, with and without a
   release (simulated).

## Outcome

- **Layout of `site/`.** `src/i18n/` (copy and page URLs), `src/layouts/Base.astro` (head, header,
  language suggestion, footer), `src/components/` (Home, Download, Releases and small pieces),
  `src/pages/` and `src/pages/pt/` (one thin route per page and language), `src/styles/global.css`,
  `public/` (screenshots, `theme.js`, `app.js`). Astro is pinned to an exact version in
  `package.json`, with `package-lock.json`.
- **Single sources.** The fonts and the icon are imported from `crates/falog-desktop/assets` and the
  roadmap from `.spec/roadmap.md` (`src/lib/roadmap.ts`) at build time; nothing is copied into the
  repository. The folder of each roadmap task's link (`backlog`, `doing`, `done`) is its stage, and
  the "Later, unordered:" sentence becomes its own group. The Portuguese copy translates roadmap text
  keyed by the exact English text, so a row edited later shows in English until translated.
- **No inline code.** The pages carry a Content Security Policy that allows only same-origin scripts,
  styles, fonts and images and connections to the GitHub API. `theme.js` (theme before the first
  paint) and `app.js` (downloads, tabs, language suggestion, releases) are plain files in `public/`;
  their localized strings are a JSON block in each page. Astro is set never to inline stylesheets.
- **Releases at runtime.** Home and Download ask `GET /repos/IFafaa/falog/releases/latest`; Download
  and Releases ask `/releases?per_page=100`. The unauthenticated limit is 60 requests an hour per
  visitor IP. The download links work without JavaScript. Release notes stay in English.
- **Screenshots**: 12 WebP files (6 views × 2 themes, 2040 × 1260, about 60 KB each) in
  `site/public/img`. To refresh them, seed demo data, capture the demo window and keep the same names
  and size.
- **Develop**: `cd site`, `npm ci`, `npm run dev` (Node 22.12 or newer); `npm run build` writes
  `site/dist`. The site lives under `/falog/`, so open <http://localhost:4321/falog/>.
- **One-time setup** on GitHub: Settings › Pages › Source: GitHub Actions.
- Follow-ups: a social preview image in PNG (some sites ignore WebP for `og:image`); the demo seed
  could map its calendar emails to areas so the calendar screenshot shows area colors and no banner.
