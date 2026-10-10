// Falog's public site: the download button for the visitor's system, view tabs, theme toggle, copy
// button and the one-time language suggestion. Releases are part of the pages (read when the site is
// built), so nothing here talks to GitHub. Localized strings come from the #strings JSON block.
"use strict";

const REPO = "https://github.com/IFafaa/falog";
const LATEST = REPO + "/releases/latest/download/";

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));

const S = JSON.parse($("#strings").textContent);
const LANG = document.documentElement.lang || "en";
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

function setupKeys() {
  if (OS !== "macos") return;
  for (const kbd of $$("kbd.mod")) {
    kbd.textContent = kbd.textContent.replace("Ctrl+", "⌘").replace("Shift+", "⇧");
  }
}

// The hero button goes straight to the file for this system. The page says whether there is a
// stable release yet and which files it has (data-no-release, data-assets).
function setupHero(mine) {
  const hero = $("#hero-download");
  if (!hero) return;
  if (hero.dataset.noRelease != null) {
    hero.href = REPO + "/blob/main/docs/install/" + mine.docs + ".md";
    $("#hero-download-label").textContent = fill(S.installOn, { os: mine.name });
    $("#hero-download-meta").textContent = "";
  } else {
    const assets = hero.dataset.assets ? hero.dataset.assets.split(" ") : null;
    // A file missing from the latest release would 404: send the button to the release page.
    hero.href = !assets || assets.includes(mine.file) ? LATEST + mine.file : hero.dataset.releaseUrl;
    $("#hero-download-label").textContent = fill(S.downloadFor, { os: mine.name });
    $("#hero-download-meta").textContent = mine.meta;
  }
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

function markCard(key) {
  const card = $("#os-" + OS);
  if (!card) return;
  card.classList.add("current");
  card.style.order = "-1";
  $("h2", card).append(" ", el("span", { class: "your-system" }, $(".platforms").dataset.yourSystem));
  if (key === "macos-x64") {
    // Make Intel the primary button on an Intel Mac.
    $("#mac-intel").classList.replace("ghost", "primary");
    $("#mac-arm").classList.replace("primary", "ghost");
  }
}

async function setupSystem() {
  if (!OS) return;
  const key = OS === "macos" ? await macArch() : OS;
  setupHero(DOWNLOADS[key]);
  markCard(key);
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

// ---- start ----------------------------------------------------------------------------------

setupKeys();
setupLanguage();
setupTheme();
setupTabs();
setupCopy();
setupSystem();
