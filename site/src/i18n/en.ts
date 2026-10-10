// English copy. Strings may hold a little inline HTML (<b>, <kbd>, <a>); they are ours, never user
// input. The Portuguese file (pt.ts) has the same shape.
//
// The site is for people deciding whether to use Falog: say what each thing does for them, in as few
// words as possible, and leave out how it is built.

const REPO = "https://github.com/IFafaa/falog";

const en = {
  lang: "en",
  htmlLang: "en",
  dateLocale: "en",
  languageName: "English",

  meta: {
    home: {
      title: "Falog: a personal organizer you run by voice",
      description:
        "Say what you need to do and Falog puts it on your list, next to your meetings. Free for Windows, macOS and Linux.",
    },
    download: {
      title: "Download Falog for Windows, macOS and Linux",
      description: "Get the latest version of Falog for Windows, macOS or Linux.",
    },
    releases: {
      title: "Falog releases",
      description: "Every version of Falog and what changed in it.",
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
    title: "Get organized<br>just by talking.",
    lede: "Falog is a personal organizer you run by voice. Say what needs doing and it's on your list, right next to your meetings.",
    download: "Download Falog",
    tour: "See what it does",
    also: "Windows · macOS · Linux · ",
    allDownloads: "all downloads",
    heroAlt: {
      dark: "Falog's task board in dark theme, with the assistant turning one spoken message into two tasks.",
      light: "Falog's task board in light theme, with the assistant turning one spoken message into two tasks.",
    },

    featuresLabel: "01 · Features",
    featuresTitle: "Everything you have to do, in one place",
    featuresLede: "Work and personal life side by side. See your tasks as a board, a list, by urgency or on the calendar.",
    viewsLabel: "Views",
    views: [
      {
        id: "board",
        name: "Board",
        caption: "<b>Board.</b> Your tasks in columns by status. Drag a card to move it along.",
        alt: "The board, with a task open on the side.",
      },
      {
        id: "list",
        name: "List",
        caption: "<b>List.</b> Every task in one compact list, sorted the way you need.",
        alt: "The list: one row per task.",
      },
      {
        id: "focus",
        name: "Focus",
        caption: "<b>Focus.</b> What needs you now, from overdue to due this week.",
        alt: "Focus: open tasks grouped by how soon they are due.",
      },
      {
        id: "calendar",
        name: "Calendar",
        caption: "<b>Calendar.</b> Your meetings and your tasks side by side, by week or month.",
        alt: "The calendar: a month of meetings and tasks.",
      },
      {
        id: "archive",
        name: "Archive",
        caption: "<b>Archive.</b> Finished tasks out of the way, one click from coming back.",
        alt: "The archive: finished tasks grouped by month.",
      },
    ],
    themeAlt: { dark: "Dark theme. ", light: "Light theme. " },

    featuresList: "What Falog does",
    features: [
      {
        icon: "sparkle",
        title: "Talk to your tasks",
        text: "Tell the assistant what you need, in your own words. It creates, updates and reviews your tasks for you, several at once if you like.",
      },
      {
        icon: "mic",
        title: "Safe, offline dictation",
        text: "Hold <kbd class=\"mod\">Ctrl+Space</kbd> and speak. Dictation works without an internet connection and stays on your computer, so it's completely safe.",
      },
      {
        icon: "agents",
        title: "Works with your AI",
        text: "Use the assistant you prefer: Claude Code, Gemini CLI, Codex or any ACP agent. You can even add your own.",
      },
      {
        icon: "folder",
        title: "Contexts kept apart",
        text: "Work, home, studies: each part of your life gets its own area and color. Focus on one, or see them all together.",
      },
      {
        icon: "calendar",
        title: "Calendar",
        text: "Your meetings right next to your tasks, by week or month.",
      },
      {
        icon: "lock",
        title: "Your data stays with you",
        text: "No account needed. Falog only goes online for the AI assistant you choose and the calendars you connect; everything else stays with you.",
      },
      {
        icon: "monitor",
        title: "Windows, macOS and Linux",
        text: "Available on all three. Opens with your computer and follows your light or dark theme.",
      },
      {
        icon: "code",
        title: "Free and open source",
        text: "Free to use, and open for anyone to read and improve.",
      },
    ],

    progressLabel: "02 · Progress",
    progressTitle: "Follow along",
    progressLede: `What's new and what's coming next. <a href="${REPO}">Follow Falog on GitHub</a>.`,
    latestTitle: "Latest release",
    latestLoading: "Checking for the latest release…",
    allReleases: "All releases",
    roadmapTitle: "Roadmap",
    roadmapLink: "Full roadmap on GitHub ↗",
    stages: { next: "Next steps", done: "Shipped" },
    moreShipped: (n: number) => `and ${n} more`,
  },

  download: {
    label: "Windows · macOS · Linux",
    title: "Get Falog",
    stableDefault: "The latest stable version.",
    noRelease: `The first version is almost here. Until then, you can build Falog yourself on <a href="${REPO}/blob/main/docs/install/windows.md">Windows</a>, <a href="${REPO}/blob/main/docs/install/macos.md">macOS</a> or <a href="${REPO}/blob/main/docs/install/linux.md">Linux</a>.`,
    yourSystem: "Your system",
    windows: {
      sub: "Installer",
      button: "Download .exe",
      note: "If Windows shows a SmartScreen warning, choose <b>More info › Run anyway</b>.",
    },
    macos: {
      sub: "Apple silicon and Intel",
      arm: "Apple silicon",
      intel: "Intel",
      note: "If macOS says it can't verify Falog, open <b>System Settings › Privacy &amp; Security</b> and click <b>Open Anyway</b>.",
    },
    linux: {
      sub: "One command, or the archive",
      copy: "Copy the install command",
      button: "Download .tar.gz",
      note: "Paste the command in a terminal, or download the archive.",
    },
    agents: "To talk to Falog, install an AI assistant: Claude Code, Gemini CLI, Codex or any ACP agent.",
    previewBadge: "Preview",
    previewText: "An early look at what's next. Less stable, for those who like to try things first.",
    older: "Looking for an older version? See <a href=\"{releases}\">all releases</a>.",
  },

  releases: {
    label: "Changelog",
    title: "Releases",
    lede: "Every version of Falog, newest first. Previews are early, less stable versions.",
    onGithub: "Releases on GitHub ↗",
  },

  footer: {
    license: "MIT License",
    source: "Source",
    releases: "Releases",
    issues: "Issues",
    specs: "Specs",
    credits: "No cookies, no trackers.",
  },

  // Read by app.js through a JSON block in the page.
  js: {
    downloadFor: "Download for {os}",
    alsoFor: "Also for ",
    installOn: "Install on {os}",
    howToInstall: "How to install",
    stableNone: "No stable version yet.",
    stable: "Stable",
    latestStable: "Latest stable",
    preview: "Preview",
    releaseNotes: "What changed",
    releaseNotesLink: "what changed ↗",
    releasePage: "Release page ↗",
    missingAsset: "Not in {version} yet; see the release page",
    loading: "Loading releases…",
    none: "No releases yet. The first one is on its way.",
    failed: "Releases couldn't be loaded right now. They're one click away below.",
    latestNone: "The first release is on its way.",
    latestFailed: "The latest release couldn't be loaded right now.",
    copied: "Copied",
    themeToLight: "Switch to light theme",
    themeToDark: "Switch to dark theme",
    arm: "Apple silicon",
    intel: "Intel",
  },

  // How the roadmap's English text (.spec/roadmap.md, written for contributors) reads on the site,
  // keyed by that exact text. Text with no entry is shown as written.
  roadmap: {
    "System tray": "Run in the background",
    "Keep Falog running without a window in the taskbar": "Close the window and Falog keeps going",
    "Quick capture hotkey": "Quick capture",
    "Get a Windows notification before things are late": "A heads-up before something is late",
    "File a task from anywhere without the assistant": "Add a task from anywhere, in a keystroke",
    "Release downloads": "Downloads for every system",
    "Google Calendar through secret iCal links": "Connect Google Calendar with a link",
    "Assistant agents": "Choose your AI assistant",
    "Calendar view with Google Calendar": "A calendar with your meetings",
    "Live transcription": "See your words as you speak",
    "Assistant threads": "Separate conversations",
    "Assistant panel with voice": "Talk to Falog",
  } as Record<string, string>,
};

export default en;
export type Copy = typeof en;
