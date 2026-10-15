// The releases, read from the GitHub API once when the site is built and rendered into the pages, so
// visitors never wait on (or get rate limited by) the API. cd.yml rebuilds the site after every push
// to main that passes CI, when a release is published and once a day.
//
// With GITHUB_TOKEN set (CI) a failed request fails the build, so a broken list is never deployed.
// Without it (local builds) the pages are built without releases and say where to find them.

const API = "https://api.github.com/repos/IFafaa/falog";
export const RELEASES_PAGE = "https://github.com/IFafaa/falog/releases";
/** Asset names every release carries, so `releases/latest/download/<name>` never changes. */
export const LATEST_DOWNLOAD = `${RELEASES_PAGE}/latest/download/`;

export interface Release {
  id: number;
  tag: string;
  title: string;
  published: string;
  url: string;
  prerelease: boolean;
  notes: string;
  assets: { name: string; url: string }[];
}

export interface Releases {
  /** False when the API could not be read (a local build without a token). */
  known: boolean;
  /** Newest first, drafts left out. */
  all: Release[];
  /** The release `releases/latest` points at. */
  stable?: Release;
  /** The newest preview, when it is newer than the stable release. */
  preview?: Release;
}

/** Only https links survive into the page. */
function https(url: unknown): string | undefined {
  return typeof url === "string" && /^https:\/\/[^\s"<>]+$/.test(url) ? url : undefined;
}

// Shape of the fields read from the GitHub REST API.
interface ApiRelease {
  id: number;
  tag_name: string;
  name: string | null;
  published_at: string | null;
  html_url: string;
  prerelease: boolean;
  draft: boolean;
  body: string | null;
  assets: { name: string; browser_download_url: string }[];
}

function toRelease(r: ApiRelease): Release {
  return {
    id: r.id,
    tag: r.tag_name,
    title: (r.name ?? "").trim(),
    published: r.published_at ?? "",
    url: https(r.html_url) ?? RELEASES_PAGE,
    prerelease: r.prerelease,
    notes: r.body ?? "",
    assets: r.assets
      .map((a) => ({ name: a.name, url: https(a.browser_download_url) }))
      .filter((a): a is { name: string; url: string } => !!a.url && a.url.startsWith(RELEASES_PAGE + "/")),
  };
}

async function get(path: string, token: string | undefined): Promise<Response> {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (token) headers.Authorization = `Bearer ${token}`;
  return fetch(API + path, { headers, signal: AbortSignal.timeout(20_000) });
}

async function load(): Promise<Releases> {
  const token = process.env.GITHUB_TOKEN || undefined;
  try {
    const [list, latest] = await Promise.all([get("/releases?per_page=100", token), get("/releases/latest", token)]);
    if (!list.ok) throw new Error(`GET /releases answered ${list.status}`);
    // 404: no stable release yet.
    if (!latest.ok && latest.status !== 404) throw new Error(`GET /releases/latest answered ${latest.status}`);
    const all = ((await list.json()) as ApiRelease[]).filter((r) => !r.draft).map(toRelease);
    const stableId = latest.ok ? ((await latest.json()) as ApiRelease).id : undefined;
    const stable = all.find((r) => r.id === stableId);
    const newest = all.find((r) => r.prerelease);
    const preview = newest && (!stable || newest.published > stable.published) ? newest : undefined;
    return { known: true, all, stable, preview };
  } catch (error) {
    if (token) throw new Error(`Could not read the releases from GitHub: ${error}`);
    console.warn(`[releases] built without releases (${error}); set GITHUB_TOKEN to include them.`);
    return { known: false, all: [] };
  }
}

export const releases: Releases = await load();

/** Whether `file` (a fixed asset name) is in the stable release; true when unknown. */
export function hasAsset(file: string): boolean {
  return !releases.stable || releases.stable.assets.some((asset) => asset.name === file);
}
