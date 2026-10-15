import { describe, expect, it } from "vitest";
import { parseRoadmap, roadmap } from "./roadmap";

const SPEC = [
  "# Roadmap",
  "",
  "| # | Task | Why |",
  "|---|---|---|",
  "| 0001 | [System tray](tasks/backlog/0001-system-tray.md) | Keep Falog running |",
  "| 0002 | [Reminders](tasks/doing/0002-reminders.md) | A heads-up, before `main` |",
  "| 0003 | [App icon](tasks/done/0003-icon.md) | A distinctive icon |",
  "| 0004 | [Someday](tasks/later/0004-someday.md) | Not a stage |",
  "| 05 | [Short number](tasks/backlog/05-short.md) | Not a task number |",
  "",
  "Later, unordered: tags, undo.",
  "",
  "Shelved: [accounts and sync](tasks/shelved/0020-accounts-and-sync.md) (0020). Falog stays on one computer.",
].join("\r\n");

describe("parseRoadmap", () => {
  const { tasks } = parseRoadmap(SPEC);

  it("reads each table row with the stage of its task's folder", () => {
    expect(tasks).toEqual([
      { num: "0001", title: "System tray", why: "Keep Falog running", stage: "next" },
      { num: "0002", title: "Reminders", why: "A heads-up, before `main`", stage: "doing" },
      { num: "0003", title: "App icon", why: "A distinctive icon", stage: "done" },
    ]);
  });

  it("ignores lines that are not task rows, such as the shelved line", () => {
    expect(tasks.map((task) => task.num)).not.toContain("0020");
    expect(parseRoadmap("Shelved: [x](tasks/shelved/0020-x.md)").tasks).toEqual([]);
  });

  it("ignores rows linking to folders that are not a stage", () => {
    expect(tasks.map((task) => task.num)).not.toContain("0004");
  });
});

describe("the roadmap built into the site", () => {
  it("is read from .spec/roadmap.md", () => {
    expect(roadmap.tasks.length).toBeGreaterThan(0);
    for (const task of roadmap.tasks) {
      expect(task.num).toMatch(/^\d{4}$/);
      expect(["doing", "next", "done"]).toContain(task.stage);
      expect(task.title).not.toBe("");
    }
  });
});
