// Falog's public site: download links for the visitor's system, view tabs, theme toggle, the one-time
// language suggestion, and the releases read from the GitHub API. Text from GitHub is only ever
// inserted as text nodes, never as HTML. Localized strings come from the #strings JSON block.
"use strict";

const REPO = "https://github.com/IFafaa/falog";
const API = "https://api.github.com/repos/IFafaa/falog";
const LATEST = REPO + "/releases/latest/download/";

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));

const S = JSON.parse($("#strings").textContent);
const LANG = document.documentElement.lang || "en";
const RELEASES_URL = document.body.dataset.releasesUrl;
const DOWNLOAD_URL = document.body.dataset.downloadUrl;

// Fixed asset names on every release, so these links always point at the latest stable build.
const DOWNLOADS = {
  windows: { name: "Windows", file: "Falog-Setup-x64.exe", meta: "x64", docs: "windows" },
  "macos-arm": { name: "macOS", file: "Falog-macOS-arm64.dmg", meta: S.arm, docs: "macos" },
  "macos-x64": { name: "macOS", file: "Falog-macOS-x64.dmg", meta: S.intel, docs: "macos" },
  linux: { name: "Linux", file: "falog-linux-x86_64.tar.gz", meta: "x86_64", docs: "linux" },
};

function fill(template, values) {
  return template.replace(/\{(\w+)\}/g, (_, key) => values[key] ?? "");
}

function el(tag, attrs, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs || {})) {
    if (value != null) node.setAttribute(key, value);
  }
  for (const child of children) {
    if (child != null) node.append(child);
  }
  return node;
}

function formatDate(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleDateString(LANG, { year: "numeric", month: "short", day: "numeric" });
}

function safeUrl(url) {
  return typeof url === "string" && /^https:\/\/[^\s"<>]+$/.test(url) ? url : null;
}

function storageGet(key) {
  try {
    return localStorage.getItem(key);
  } catch (e) {
    return null; // Storage blocked (private window, strict settings).
  }
}

function storageSet(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch (e) {
    // Not remembered; the choice still applies to this visit.
  }
}

// ---- the visitor's system --------------------------------------------------------------------

function detectOs() {
  const platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
  const ua = navigator.userAgent || "";
  if (/android|iphone|ipad|ipod/i.test(ua) || /android|ios/i.test(platform)) return null;
  if (/win/i.test(platform) || /windows/i.test(ua)) return "windows";
  if (/mac/i.test(platform) || /mac os x/i.test(ua)) {
    // iPads report a Mac platform; a touch screen gives them away.
    return navigator.maxTouchPoints > 1 ? null : "macos";
  }
  if (/linux|x11/i.test(platform) || /linux/i.test(ua)) return "linux";
  return null;
}

// Apple silicon unless the browser says Intel; only Chromium browsers can tell.
async function macArch() {
  try {
    const data = await navigator.userAgentData.getHighEntropyValues(["architecture"]);
    return data.architecture === "x86" ? "macos-x64" : "macos-arm";
  } catch (e) {
    return "macos-arm";
  }
}

const OS = detectOs();
let mine = null; // The DOWNLOADS entry for this visitor, once known.

function setupKeys() {
  if (OS !== "macos") return;
  for (const kbd of $$("kbd.mod")) {
    kbd.textContent = kbd.textContent.replace("Ctrl+", "⌘").replace("Shift+", "⇧");
  }
}

async function setupSystem() {
  if (!OS) return;
  const key = OS === "macos" ? await macArch() : OS;
  mine = DOWNLOADS[key];

  const hero = $("#hero-download");
  if (hero) {
    hero.href = LATEST + mine.file;
    $("#hero-download-label").textContent = fill(S.downloadFor, { os: mine.name });
    $("#hero-download-meta").textContent = mine.meta;
    const names = { windows: "Windows", macos: "macOS", linux: "Linux" };
    const also = $("#hero-also");
    also.replaceChildren(S.alsoFor);
    ["windows", "macos", "linux"]
      .filter((o) => o !== OS)
      .forEach((o, i) => {
        if (i) also.append(" · ");
        also.append(el("a", { href: DOWNLOAD_URL + "#os-" + o }, names[o]));
      });
  }

  const card = $("#os-" + OS);
  if (card) {
    card.classList.add("current");
    card.style.order = "-1";
    $("h2", card).append(" ", el("span", { class: "your-system" }, $(".platforms").dataset.yourSystem));
    if (key === "macos-x64") {
      // Make Intel the primary button on an Intel Mac.
      $("#mac-intel").classList.replace("ghost", "primary");
      $("#mac-arm").classList.replace("primary", "ghost");
    }
  }
}

// ---- language -------------------------------------------------------------------------------

// Suggests the other language once, when the browser's first choice is that language and the
// visitor has not picked one; never redirects.
function setupLanguage() {
  for (const link of $$("[data-lang-choice]")) {
    link.addEventListener("click", () => storageSet("falog-lang", link.dataset.langChoice));
  }
  const banner = $("#lang-suggest");
  if (!banner || storageGet("falog-lang")) return;
  const preferred = ((navigator.languages && navigator.languages[0]) || navigator.language || "").toLowerCase();
  const prefersPt = preferred.startsWith("pt");
  const onPt = LANG.toLowerCase().startsWith("pt");
  if (prefersPt === onPt) return;
  banner.hidden = false;
  $("#lang-suggest-dismiss").addEventListener("click", () => {
    storageSet("falog-lang", onPt ? "pt" : "en");
    banner.hidden = true;
  });
}

// ---- theme ----------------------------------------------------------------------------------

function setupTheme() {
  const root = document.documentElement;
  const button = $("#theme-toggle");
  const label = () => {
    const text = root.dataset.theme === "light" ? S.themeToDark : S.themeToLight;
    button.setAttribute("aria-label", text);
    button.title = text;
  };
  label();
  button.addEventListener("click", () => {
    root.dataset.theme = root.dataset.theme === "light" ? "dark" : "light";
    storageSet("falog-theme", root.dataset.theme);
    label();
  });
  // Follow the system while the visitor has not picked a theme.
  if (window.matchMedia) {
    matchMedia("(prefers-color-scheme: light)").addEventListener("change", (event) => {
      if (!storageGet("falog-theme")) {
        root.dataset.theme = event.matches ? "light" : "dark";
        label();
      }
    });
  }
}

// ---- view tabs ------------------------------------------------------------------------------

function setupTabs() {
  const tabs = $$('[role="tab"]');
  const select = (tab, focus) => {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      document.getElementById(other.getAttribute("aria-controls")).hidden = !selected;
    }
    if (focus) tab.focus();
  };
  tabs.forEach((tab, i) => {
    tab.addEventListener("click", () => select(tab, false));
    tab.addEventListener("keydown", (event) => {
      const moves = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 };
      if (!(event.key in moves)) return;
      event.preventDefault();
      select(tabs[(moves[event.key] + tabs.length) % tabs.length], true);
    });
  });
}

// ---- copy button ----------------------------------------------------------------------------

function setupCopy() {
  for (const button of $$("[data-copy]")) {
    button.addEventListener("click", async () => {
      const source = document.getElementById(button.dataset.copy);
      try {
        await navigator.clipboard.writeText(source.textContent.trim());
      } catch (e) {
        // No clipboard access: select the text so Ctrl+C works.
        const range = document.createRange();
        range.selectNodeContents(source);
        const selection = getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        return;
      }
      const label = button.getAttribute("aria-label");
      button.classList.add("done");
      button.setAttribute("aria-label", S.copied);
      setTimeout(() => {
        button.classList.remove("done");
        button.setAttribute("aria-label", label);
      }, 1600);
    });
  }
}

// ---- markdown, as text ----------------------------------------------------------------------

// Inline markdown: `code`, **bold**, [text](https://...) and bare https links. Everything else stays
// literal text, so a release note can never inject markup.
function inline(text) {
  const nodes = [];
  const pattern = /`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\(([^)\s]+)\)|(https:\/\/[^\s)<>]+)/g;
  let last = 0;
  let match;
  while ((match = pattern.exec(text))) {
    if (match.index > last) nodes.push(text.slice(last, match.index));
    if (match[1] != null) {
      nodes.push(el("code", null, match[1]));
    } else if (match[2] != null) {
      nodes.push(el("strong", null, match[2]));
    } else if (match[3] != null) {
      const url = safeUrl(match[4]);
      nodes.push(url ? el("a", { href: url, rel: "nofollow" }, match[3]) : match[3]);
    } else {
      nodes.push(el("a", { href: match[5], rel: "nofollow" }, match[5]));
    }
    last = pattern.lastIndex;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

function markdown(source) {
  const root = el("div", { class: "notes", lang: "en" });
  let list = null;
  let paragraph = null;
  for (const raw of source.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trim();
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    const item = /^[-*+]\s+(.*)$/.exec(line);
    if (!line || heading || item) paragraph = null;
    if (!item) list = null;
    if (!line || /^<!--.*-->$/.test(line)) continue;
    if (heading) {
      root.append(el("h4", null, ...inline(heading[1])));
    } else if (item) {
      if (!list) root.append((list = el("ul")));
      list.append(el("li", null, ...inline(item[1])));
    } else if (paragraph) {
      paragraph.append(" ", ...inline(line));
    } else {
      root.append((paragraph = el("p", null, ...inline(line))));
    }
  }
  return root;
}

// ---- releases -------------------------------------------------------------------------------

async function getJson(url) {
  const response = await fetch(url, { headers: { Accept: "application/vnd.github+json" } });
  if (!response.ok) {
    const error = new Error("GitHub answered " + response.status);
    error.status = response.status;
    throw error;
  }
  return response.json();
}

const releasePage = (release) => safeUrl(release.html_url) || REPO + "/releases";

function releaseItem(release, latestId, open) {
  const badge = release.prerelease
    ? el("span", { class: "badge preview-badge" }, S.preview)
    : el("span", { class: "badge stable" }, release.id === latestId ? S.latestStable : S.stable);
  const item = el(
    "li",
    null,
    el(
      "div",
      { class: "release-head" },
      el("a", { href: releasePage(release) }, release.tag_name),
      badge,
      el("time", { class: "release-date", datetime: release.published_at }, formatDate(release.published_at)),
    ),
  );
  const title = (release.name || "").trim();
  if (title && title !== release.tag_name) item.append(el("p", { class: "release-title", lang: "en" }, title));
  if ((release.body || "").trim()) {
    const details = el("details", { class: "release-notes" }, el("summary", null, S.releaseNotes), markdown(release.body));
    details.open = open;
    item.append(details);
  }
  return item;
}

// The hero and the download buttons once the latest stable release is known.
function showStable(release) {
  const version = $("#hero-version");
  if (version) version.textContent = fill(S.versionOut, { version: release.tag_name });

  const status = $("#stable-status");
  if (status) {
    status.replaceChildren(
      el("span", { class: "badge stable" }, S.stable),
      " " + release.tag_name + " · " + formatDate(release.published_at) + " · ",
      el("a", { href: releasePage(release) }, S.releaseNotesLink),
    );
  }

  // A file missing from this release would 404: send that button to the release page instead.
  const files = new Set((release.assets || []).map((asset) => asset.name));
  if (files.size) {
    for (const link of $$("a.dl, #hero-download")) {
      const file = link.href.startsWith(LATEST) ? link.href.slice(LATEST.length) : null;
      if (file && !files.has(file)) {
        link.href = releasePage(release);
        link.title = fill(S.missingAsset, { version: release.tag_name });
      }
    }
  }

  const latest = $("#latest-release");
  if (latest) latest.replaceChildren(el("ol", { class: "releases" }, releaseItem(release, release.id, false)));
}

function showNoRelease() {
  document.body.classList.add("no-release");
  const notice = $("#no-release");
  if (notice) notice.hidden = false;
  const status = $("#stable-status");
  if (status) status.textContent = S.stableNone;
  const version = $("#hero-version");
  if (version) version.textContent = S.comingSoon;
  const hero = $("#hero-download");
  if (hero) {
    hero.href = mine ? REPO + "/blob/main/docs/install/" + mine.docs + ".md" : DOWNLOAD_URL;
    $("#hero-download-label").textContent = mine ? fill(S.buildFor, { os: mine.name }) : S.howToInstall;
    $("#hero-download-meta").textContent = "";
  }
  const latest = $("#latest-release");
  if (latest) latest.replaceChildren(el("p", { class: "muted" }, S.latestNone));
}

function showPreview(release) {
  $("#preview").hidden = false;
  $("#preview-version").textContent = release.tag_name + " · " + formatDate(release.published_at);
  const list = $("#preview-assets");
  list.replaceChildren();
  for (const asset of release.assets || []) {
    const url = safeUrl(asset.browser_download_url);
    if (url && url.startsWith(REPO + "/")) list.append(el("li", null, el("a", { href: url }, asset.name)));
  }
  list.append(el("li", null, el("a", { href: releasePage(release) }, S.releasePage)));
}

function showReleaseList(releases) {
  const list = $("#release-list");
  const shown = releases.filter((r) => !r.draft);
  if (!shown.length) {
    list.replaceChildren(el("li", { class: "muted" }, S.none));
    return;
  }
  const latest = shown.find((r) => !r.prerelease);
  list.replaceChildren(...shown.map((release, i) => releaseItem(release, latest && latest.id, i === 0)));
}

async function loadReleases() {
  const page = document.body.dataset.page;
  const wantsList = page === "download" || page === "releases";
  const [latest, all] = await Promise.allSettled([
    page === "releases" ? Promise.resolve(null) : getJson(API + "/releases/latest"),
    wantsList ? getJson(API + "/releases?per_page=100") : Promise.resolve(null),
  ]);

  let stable = null;
  if (page !== "releases") {
    if (latest.status === "fulfilled") {
      stable = latest.value;
      showStable(stable);
    } else if (latest.reason && latest.reason.status === 404) {
      showNoRelease();
    } else {
      // Offline or rate limited: the static links already point at the latest release.
      const card = $("#latest-release");
      if (card) card.replaceChildren(el("p", { class: "muted" }, S.latestFailed));
    }
  }
  if (!wantsList) return;

  if (all.status !== "fulfilled" || !Array.isArray(all.value)) {
    const list = $("#release-list");
    if (list) list.replaceChildren(el("li", { class: "muted" }, S.failed));
    return;
  }
  if (page === "releases") {
    showReleaseList(all.value);
  } else {
    const preview = all.value.find((r) => r.prerelease && !r.draft);
    if (preview && (!stable || new Date(preview.published_at) > new Date(stable.published_at))) showPreview(preview);
  }
}

// ---- start ----------------------------------------------------------------------------------

setupKeys();
setupLanguage();
setupTheme();
setupTabs();
setupCopy();
setupSystem().finally(loadReleases);
