// Reads the roadmap from .spec/roadmap.md when the site is built, so the page always shows the
// roadmap of the commit it was built from. The folder of each task's link (tasks/doing, tasks/backlog,
// tasks/done) is where it stands; the "Later, unordered:" sentence lists ideas without a task yet.

import roadmapMarkdown from "../../../.spec/roadmap.md?raw";

export type Stage = "doing" | "next" | "done";

export interface RoadmapTask {
  num: string;
  title: string;
  why: string;
  stage: Stage;
}

export interface Roadmap {
  tasks: RoadmapTask[];
  later: string[];
}

const STAGES: Record<string, Stage> = { doing: "doing", backlog: "next", done: "done" };

/** Splits on commas that are not inside parentheses: "a, b (c, d)" is ["a", "b (c, d)"]. */
function splitList(text: string): string[] {
  const items: string[] = [];
  let depth = 0;
  let current = "";
  for (const char of text) {
    if (char === "(") depth++;
    if (char === ")") depth--;
    if (char === "," && depth === 0) {
      items.push(current.trim());
      current = "";
    } else {
      current += char;
    }
  }
  if (current.trim()) items.push(current.trim());
  return items;
}

export function parseRoadmap(markdown: string): Roadmap {
  const tasks: RoadmapTask[] = [];
  const row = /^\|\s*(\d{4})\s*\|\s*\[([^\]]+)\]\(tasks\/(\w+)\/[^)]*\)\s*\|\s*(.*?)\s*\|\s*$/;
  for (const line of markdown.split(/\r?\n/)) {
    const match = row.exec(line);
    const stage = match && STAGES[match[3]];
    if (match && stage) tasks.push({ num: match[1], title: match[2], why: match[4], stage });
  }
  const marker = "Later, unordered:";
  const start = markdown.indexOf(marker);
  const sentence =
    start < 0
      ? ""
      : markdown
          .slice(start + marker.length)
          .split(/\r?\n\s*\r?\n/)[0]
          .replace(/\s+/g, " ")
          .trim()
          .replace(/\.$/, "");
  return { tasks, later: sentence ? splitList(sentence) : [] };
}

export const roadmap: Roadmap = parseRoadmap(roadmapMarkdown);
