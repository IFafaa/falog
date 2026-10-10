// English copy. Strings may hold a little inline HTML (<b>, <kbd>, <a>); they are ours, never user
// input. The Portuguese file (pt.ts) has the same shape.

const REPO = "https://github.com/IFafaa/falog";

const en = {
  lang: "en",
  htmlLang: "en",
  dateLocale: "en",
  languageName: "English",

  meta: {
    home: {
      title: "Falog: a personal organizer you talk to",
      description:
        "Falog keeps your tasks, errands and appointments in one place, on your computer. Tell it what you need to do, by voice or text, and it files it for you. Free and open source for Windows, macOS and Linux.",
    },
    download: {
      title: "Download Falog for Windows, macOS and Linux",
      description: "Get the latest stable Falog for Windows, macOS or Linux, or try a preview build.",
    },
    releases: {
      title: "Falog releases and release notes",
      description: "Every Falog release, stable and preview, with its release notes.",
    },
  },

  nav: {
    skip: "Skip to content",
    sections: "Pages",
    home: "Overview",
    download: "Download",
    releases: "Releases",
    github: "GitHub",
    switchLanguage: "Ver em português",
    themeToLight: "Switch to light theme",
    themeToDark: "Switch to dark theme",
    brandHome: "Falog, home page",
  },

  // Shown once, on the other language's pages, to visitors whose browser prefers this language.
  suggest: {
    text: "This page is also available in English.",
    action: "View in English",
    dismiss: "Not now",
  },

  home: {
    eyebrow: "Free and open source",
    title: "A personal organizer<br>you talk to.",
    lede:
      "Tasks, errands and appointments in one place, on your computer. Say what you need to do, by voice or text, and Falog files it with the area, the due date, the priority and who asked.",
    download: "Download Falog",
    source: "View the source",
    also: "Windows · macOS · Linux · ",
    allDownloads: "all downloads",
    heroAlt: {
      dark: "Falog in dark theme: the board with To do, In progress and Waiting columns, and the assistant dock where one spoken message became two tasks.",
      light:
        "Falog in light theme: the board with To do, In progress and Waiting columns, and the assistant dock where one spoken message became two tasks.",
    },

    featuresLabel: "01 · Features",
    featuresTitle: "Everything you have to do, in one window",
    featuresLede:
      "Work and personal life side by side, never siloed. Five views of the same tasks, each one keystroke away.",
    viewsLabel: "Views",
    views: [
      {
        id: "board",
        name: "Board",
        caption:
          "<b>Board.</b> Columns for To do, In progress, Waiting and Done. Cards show the area, priority, due date and who asked; drag one to change its status, click it to see its notes.",
        alt: "The board with a task open in the side panel, showing its description and notes.",
      },
      {
        id: "list",
        name: "List",
        caption:
          "<b>List.</b> Every open task in one dense table, sorted by urgency, due date or priority, with completed ones a toggle away.",
        alt: "The list view: one row per task with status, title, area, priority and due date.",
      },
      {
        id: "focus",
        name: "Focus",
        caption:
          "<b>Focus.</b> What needs you now: overdue, due today, due this week, in progress, waiting on others, upcoming and backlog. The answer to “what was I supposed to do?” in seconds.",
        alt: "The Focus view: open tasks grouped under Overdue, Due today, Due this week and more.",
      },
      {
        id: "calendar",
        name: "Calendar",
        caption:
          "<b>Calendar.</b> Your meetings by week or month, colored by area, with the tasks due each day in the same grid.",
        alt: "The calendar month view: meetings from work and personal calendars and the tasks due each day.",
      },
      {
        id: "archive",
        name: "Archive",
        caption:
          "<b>Archive.</b> Done tasks put away by the month you finished them, so the board stays short. Restore any of them with one click.",
        alt: "The Archive view: finished tasks grouped by month of completion.",
      },
    ],
    themeAlt: { dark: "Dark theme. ", light: "Light theme. " },

    featuresList: "What Falog does",
    features: [
      {
        icon: "sparkle",
        title: "Talk to your tasks",
        text: "The assistant dock files, updates and reviews tasks from what you say. One message can hold several requests, and each becomes its own task with all the context you gave. Separate threads keep separate conversations.",
      },
      {
        icon: "mic",
        title: "Voice that stays on your computer",
        text: "Hold <kbd class=\"mod\">Ctrl+Space</kbd> and speak. Speech recognition runs on your machine and the words appear as you talk; your voice is never uploaded. The speech model downloads once, the first time you dictate.",
      },
      {
        icon: "agents",
        title: "Works with your agents",
        text: "Use the agent you already have: Claude Code, Gemini CLI, Codex or any agent that speaks the Agent Client Protocol (ACP), including one you add yourself. Pick the agent, model and effort for each conversation. Any MCP client can file and review your tasks too.",
      },
      {
        icon: "folder",
        title: "Areas for every part of life",
        text: "Work, Home, Health, Studies: name your areas and give each a color. Filter everything by area, meetings included, and still see the whole week in one place.",
      },
      {
        icon: "calendar",
        title: "Calendar",
        text: "Falog's calendar shows your meetings and the tasks due each day in one week or month view, each in its area's color. Connect Google Calendar today, by pasting a calendar's private link or by signing in, which also lets the assistant book meetings for you. More providers are coming.",
      },
      {
        icon: "lock",
        title: "Local and private",
        text: "Your tasks live in one SQLite file on your computer. No account, no server, works offline. When you use the assistant, the agent you picked reads and writes them through Falog's MCP server, and nothing else does.",
      },
      {
        icon: "monitor",
        title: "Windows, macOS and Linux",
        text: "A native app on all three, written in Rust. It starts with your computer, opens instantly in a single window and follows your light or dark system theme.",
      },
      {
        icon: "code",
        title: "Open source",
        text: "MIT licensed, with the specs and the roadmap in the repository. Read the code, open an issue, or build it yourself in a few commands.",
      },
    ],

    progressLabel: "02 · Progress",
    progressTitle: "Follow along",
    progressLede: `What shipped, what is being built and what comes next. <a href="${REPO}">Watch the repository</a> to hear about new releases.`,
    latestTitle: "Latest release",
    latestLoading: "Asking GitHub for the latest release…",
    allReleases: "All releases and their notes",
    roadmapTitle: "Roadmap",
    roadmapLink: "The roadmap in the repository ↗",
    stages: { doing: "In progress", next: "Next", done: "Shipped", later: "Later" },
    moreShipped: (n: number) => `and ${n} more`,
  },

  download: {
    label: "Windows · macOS · Linux",
    title: "Get Falog",
    stableDefault: "Stable channel: the latest release on GitHub.",
    noRelease: `There is no stable release yet; the first one is on its way. Until then, build Falog from source with the guide for <a href="${REPO}/blob/main/docs/install/windows.md">Windows</a>, <a href="${REPO}/blob/main/docs/install/macos.md">macOS</a> or <a href="${REPO}/blob/main/docs/install/linux.md">Linux</a>.`,
    yourSystem: "Your system",
    windows: {
      sub: "x64 · installer",
      button: "Download .exe",
      note: "Installs for your user, no administrator rights needed. It is not code-signed yet, so if SmartScreen says it protected your PC, choose <b>More info › Run anyway</b>.",
    },
    macos: {
      sub: "Apple silicon and Intel · disk image",
      arm: "Apple silicon",
      intel: "Intel",
      note: "Not notarized yet. If macOS says it cannot verify Falog, open <b>System Settings › Privacy &amp; Security</b> and click <b>Open Anyway</b>.",
    },
    linux: {
      sub: "x86_64 · X11 and Wayland",
      copy: "Copy the install command",
      button: "Download .tar.gz",
      note: "Run the command in a terminal, or download the archive. Nothing else to set up.",
    },
    agents: `The assistant talks through an agent: Claude Code, Gemini CLI, Codex or any ACP agent. Install the one you use and sign in once; Falog finds it. Prefer to build Falog yourself? Follow the <a href="${REPO}/tree/main/docs/install">install guides</a>.`,
    previewBadge: "Preview",
    previewText:
      "An early build of what comes next. It is less stable than the release above: expect rough edges, and keep a copy of your data before trying it.",
    older: "Looking for an older version? See <a href=\"{releases}\">all releases</a>.",
  },

  releases: {
    label: "Changelog",
    title: "Releases",
    lede: "Every Falog release, newest first. Stable releases are the ones the download page offers; previews are early builds of what comes next and less stable.",
    onGithub: "Releases on GitHub ↗",
  },

  footer: {
    license: "MIT License",
    source: "Source",
    releases: "Releases",
    issues: "Issues",
    specs: "Specs",
    credits:
      "Set in IBM Plex Sans and Lilex (SIL Open Font License), in the One Dark and One Light colors. No cookies, no trackers: the only outside request is to GitHub, for the releases.",
  },

  // Read by app.js through a JSON block in the page.
  js: {
    downloadFor: "Download for {os}",
    alsoFor: "Also for ",
    buildFor: "Build for {os}",
    howToInstall: "How to install",
    versionOut: "{version} is out · stable",
    comingSoon: "First release coming soon",
    stableNone: "Stable channel: no release yet.",
    stable: "Stable",
    latestStable: "Latest stable",
    preview: "Preview",
    releaseNotes: "Release notes",
    releaseNotesLink: "release notes ↗",
    releasePage: "Release page ↗",
    missingAsset: "Not in {version} yet; see the release page",
    loading: "Loading releases from GitHub…",
    none: "No releases yet. Watch the repository to hear about the first one.",
    failed: "GitHub did not answer just now. The releases are one click away below.",
    latestNone: "No release yet: the first one is on its way.",
    latestFailed: "GitHub did not answer just now.",
    copied: "Copied",
    themeToLight: "Switch to light theme",
    themeToDark: "Switch to dark theme",
    arm: "Apple silicon",
    intel: "Intel",
  },

  // Translations of the roadmap's English text (.spec/roadmap.md), keyed by that exact text. Text with
  // no entry, such as a row edited after the translation, is shown in English.
  roadmap: {} as Record<string, string>,
};

export default en;
export type Copy = typeof en;
