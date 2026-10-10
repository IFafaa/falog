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
- [x] Copy for people deciding to use Falog: short, about what each thing does for them, never how
      it is built (no file formats, libraries, languages or setup steps)
- [x] Home: hero with "Download for <your system>" (Apple silicon or Intel on a Mac when the browser
      tells), the views with screenshots, eight features (assistant, offline dictation, agents with
      equal weight for Claude Code, Gemini CLI, Codex and any ACP agent, separate contexts, calendar,
      data that stays with you, three systems, free and open source), the latest release and the
      roadmap: next steps and what shipped, never an empty group, without engineering-only tasks
- [x] Download: cards for Windows, macOS and Linux with the fixed asset URLs, the Linux one-liner with a
      copy button, the SmartScreen and Open Anyway notes, and the newest preview when it is newer than
      the stable release, marked less stable
- [x] Releases: every release with version, date, stable/preview badge and notes rendered as text (a
      small markdown subset parsed into data; no HTML from GitHub is ever injected)
- [x] Releases are baked in when the site is built, so a visitor's page never waits on or fails
      with the GitHub API; the site is rebuilt when a release is published and daily
- [x] No release yet, GitHub unreachable at build time, or an asset missing from the latest
      release: the pages still work, never show a spinner, and never link to a 404 they know about
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
  the site lists next steps (with their reason) and shipped work (titles only), leaving out work in
  progress, the "Later" ideas and engineering-only tasks (`BEHIND_THE_SCENES` in `Home.astro`). Each
  copy file rewrites roadmap text for visitors, keyed by the exact English text of the spec, so a row
  edited later shows as written until its entry is updated.
- **No inline code.** The pages carry a Content Security Policy that allows only same-origin scripts,
  styles, fonts and images, nothing from elsewhere. `theme.js` (theme before the first paint) and
  `app.js` (download button for the visitor's system, tabs, language suggestion) are plain files in
  `public/`; their localized strings are a JSON block in each page. Astro is set never to inline
  stylesheets.
- **Releases at build time** (`src/lib/releases.ts`). The build reads `/releases?per_page=100` and
  `/releases/latest` once and renders the stable version, the download links, the preview and the
  release list into the HTML; the pages make no request to GitHub. In CI, `GITHUB_TOKEN` (5,000
  requests an hour) is set and a failed read fails the build, so the last good site stays up. A local
  build without a token reads the API anonymously and, if that fails, builds pages that link to the
  releases on GitHub instead. `release.yml` starts `pages.yml` with `gh workflow run` after
  publishing (a release made with `GITHUB_TOKEN` starts no workflow by itself), and `pages.yml` also
  runs daily. Release notes stay in English.
- **Screenshots**: 12 WebP files (6 views × 2 themes, 2040 × 1260, about 60 KB each) in
  `site/public/img`. To refresh them, seed demo data, capture the demo window and keep the same names
  and size.
- **Develop**: `cd site`, `npm ci`, `npm run dev` (Node 22.12 or newer); `npm run build` writes
  `site/dist`. The site lives under `/falog/`, so open <http://localhost:4321/falog/>.
- **One-time setup** on GitHub: Settings › Pages › Source: GitHub Actions.
- Follow-ups: a social preview image in PNG (some sites ignore WebP for `og:image`); the demo seed
  could map its calendar emails to areas so the calendar screenshot shows area colors and no banner.
