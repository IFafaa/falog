// Reads the roadmap from .spec/roadmap.md when the site is built, so the page always shows the
// roadmap of the commit it was built from. The folder of each task's link (tasks/doing, tasks/backlog,
// tasks/done) is where it stands (the site shows what is next and what shipped).

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
}

const STAGES: Record<string, Stage> = { doing: "doing", backlog: "next", done: "done" };

export function parseRoadmap(markdown: string): Roadmap {
  const tasks: RoadmapTask[] = [];
  const row = /^\|\s*(\d{4})\s*\|\s*\[([^\]]+)\]\(tasks\/(\w+)\/[^)]*\)\s*\|\s*(.*?)\s*\|\s*$/;
  for (const line of markdown.split(/\r?\n/)) {
    const match = row.exec(line);
    const stage = match && STAGES[match[3]];
    if (match && stage) tasks.push({ num: match[1], title: match[2], why: match[4], stage });
  }
  return { tasks };
}

export const roadmap: Roadmap = parseRoadmap(roadmapMarkdown);
