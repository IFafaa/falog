import { afterEach, describe, expect, it, vi } from "vitest";

const PAGE = "https://github.com/IFafaa/falog/releases";

/** How the GitHub API answers one request: a status and a JSON body, or a failed connection. */
type Answer = { status: number; body?: unknown } | Error;

/** A release as the GitHub REST API lists it. */
function release(id: number, tag: string, published: string, extra: Record<string, unknown> = {}) {
  return {
    id,
    tag_name: tag,
    name: ` Falog ${tag.slice(1)} `,
    published_at: published,
    html_url: `${PAGE}/tag/${tag}`,
    prerelease: tag.includes("-"),
    draft: false,
    body: "## Features\n- something new",
    assets: [{ name: "install.sh", browser_download_url: `${PAGE}/download/${tag}/install.sh` }],
    ...extra,
  };
}

/** Loads lib/releases.ts as a build would, with the API answering `/releases` and `/releases/latest`. */
async function build(list: Answer, latest: Answer, token = "") {
  vi.resetModules();
  vi.stubEnv("GITHUB_TOKEN", token);
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  const fetch = vi.fn(async (url: string, _init?: RequestInit) => {
    const answer = url.endsWith("/releases/latest") ? latest : list;
    if (answer instanceof Error) throw answer;
    return new Response(JSON.stringify(answer.body ?? { message: "error" }), { status: answer.status });
  });
  vi.stubGlobal("fetch", fetch);
  const module = await import("./releases");
  return { ...module, fetch, warn };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

const v020 = release(20, "v0.2.0", "2026-09-01T12:00:00Z");
const v030rc = release(29, "v0.3.0-rc.1", "2026-10-01T12:00:00Z");
const v030 = release(30, "v0.3.0", "2026-10-10T12:00:00Z");
const v040beta = release(39, "v0.4.0-beta.1", "2026-10-14T12:00:00Z");

describe("releases read at build time", () => {
  it("keeps GitHub's order, newest first, without drafts", async () => {
    const draft = release(40, "v0.4.0", "", { draft: true, published_at: null });
    const { releases } = await build({ status: 200, body: [draft, v030, v030rc, v020] }, { status: 200, body: v030 });
    expect(releases.known).toBe(true);
    expect(releases.all.map((r) => r.tag)).toEqual(["v0.3.0", "v0.3.0-rc.1", "v0.2.0"]);
    expect(releases.stable?.tag).toBe("v0.3.0");
    expect(releases.all[0]).toMatchObject({ title: "Falog 0.3.0", prerelease: false, url: `${PAGE}/tag/v0.3.0` });
    expect(releases.all[1].prerelease).toBe(true);
  });

  it("offers a preview only while it is newer than the stable release", async () => {
    const behind = await build({ status: 200, body: [v030, v030rc] }, { status: 200, body: v030 });
    expect(behind.releases.preview).toBeUndefined();
    const ahead = await build({ status: 200, body: [v040beta, v030, v030rc] }, { status: 200, body: v030 });
    expect(ahead.releases.preview?.tag).toBe("v0.4.0-beta.1");
  });

  it("has no stable release while GitHub has only previews", async () => {
    const { releases, hasAsset } = await build({ status: 200, body: [v030rc] }, { status: 404 });
    expect(releases.known).toBe(true);
    expect(releases.stable).toBeUndefined();
    expect(releases.preview?.tag).toBe("v0.3.0-rc.1");
    // Nothing is known to be missing, so the fixed download links stay.
    expect(hasAsset("Falog-Setup-x64.exe")).toBe(true);
  });

  it("knows no release at all on a new repository", async () => {
    const { releases } = await build({ status: 200, body: [] }, { status: 404 });
    expect(releases).toEqual({ known: true, all: [], stable: undefined, preview: undefined });
  });

  it("keeps only https links to the releases page", async () => {
    const odd = release(31, "v0.3.1", "2026-10-12T12:00:00Z", {
      html_url: "http://example.com/v0.3.1",
      body: null,
      assets: [
        { name: "Falog-Setup-x64.exe", browser_download_url: `${PAGE}/download/v0.3.1/Falog-Setup-x64.exe` },
        { name: "falog-linux-x86_64.tar.gz", browser_download_url: "https://example.com/falog.tar.gz" },
        { name: "install.sh", browser_download_url: "javascript:alert(1)" },
      ],
    });
    const { releases, hasAsset } = await build({ status: 200, body: [odd] }, { status: 200, body: odd });
    expect(releases.stable?.url).toBe(PAGE);
    expect(releases.stable?.notes).toBe("");
    expect(releases.stable?.assets.map((a) => a.name)).toEqual(["Falog-Setup-x64.exe"]);
    expect(hasAsset("Falog-Setup-x64.exe")).toBe(true);
    expect(hasAsset("install.sh")).toBe(false);
  });

  it("asks with the token when there is one, and anonymously otherwise", async () => {
    const anonymous = await build({ status: 200, body: [] }, { status: 404 });
    const headers = (call: unknown[]) => (call[1] as RequestInit).headers as Record<string, string>;
    expect(anonymous.fetch.mock.calls.map(headers).some((h) => "Authorization" in h)).toBe(false);
    const signed = await build({ status: 200, body: [] }, { status: 404 }, "t0ken");
    expect(signed.fetch.mock.calls.map(headers).every((h) => h.Authorization === "Bearer t0ken")).toBe(true);
  });
});

describe("when GitHub cannot be read", () => {
  // Regression: when GitHub's API refused the site (403, rate limited), the releases page stayed on
  // "loading"; a build without a token now makes pages that link to GitHub instead.
  it("builds without releases when the API is rate limited and there is no token", async () => {
    const limited = { status: 403, body: { message: "API rate limit exceeded" } };
    const { releases, hasAsset, warn } = await build(limited, limited);
    expect(releases).toEqual({ known: false, all: [] });
    expect(hasAsset("install.sh")).toBe(true);
    expect(warn).toHaveBeenCalledOnce();
  });

  it("builds without releases when GitHub is unreachable or failing and there is no token", async () => {
    const offline = await build(new TypeError("fetch failed"), new TypeError("fetch failed"));
    expect(offline.releases.known).toBe(false);
    const failing = await build({ status: 200, body: [v030] }, { status: 500 });
    expect(failing.releases.known).toBe(false);
  });

  it("fails the build when it has a token, so a broken list is never deployed", async () => {
    await expect(build({ status: 403 }, { status: 403 }, "t0ken")).rejects.toThrow(
      /Could not read the releases from GitHub/,
    );
    await expect(build({ status: 200, body: [] }, { status: 502 }, "t0ken")).rejects.toThrow(/answered 502/);
  });
});
