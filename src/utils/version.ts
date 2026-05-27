import { readStorage } from "./storage";

declare const __APP_VERSION__: string;

export const APP_VERSION = __APP_VERSION__;

export type BuildChannel = "dev" | "stable";

export const BUILD_CHANNEL: BuildChannel = import.meta.env.DEV
  ? "dev"
  : "stable";

export const GITHUB_REPO = "zium1337/HSPlanner";

const MOCK_KEY = "hsplanner.update.mock";

const MOCK_FIXTURE: GithubRelease = {
  tag_name: "v2137.0.0",
  name: "HSPlanner 2137.0.0 (mock)",
  html_url: "https://example.test/r/2137.0.0",
  published_at: new Date().toISOString(),
  body: `## New
- test

## Improved
- test

## Balance
- test

## Fixes
- test`,
  assets: [
    {
      name: "HSPlanner-2137.0.0.dmg",
      size: 12_400_000,
      digest:
        "sha256:3a8f1234567890abcdef1234567890abcdef1234567890abcdef1234d214",
      browser_download_url: "https://example.test/dl/HSPlanner-2137.0.0.dmg",
    },
  ],
};

export function isMockEnabled(): boolean {
  return readStorage(MOCK_KEY) !== null;
}

function isGithubRelease(p: unknown): p is GithubRelease {
  if (!p || typeof p !== "object") return false;
  const o = p as Record<string, unknown>;
  const optString = (v: unknown) => v === undefined || typeof v === "string";
  if (!optString(o.tag_name)) return false;
  if (!optString(o.html_url)) return false;
  if (!optString(o.name)) return false;
  if (!optString(o.body)) return false;
  if (!optString(o.published_at)) return false;
  if (o.assets !== undefined && !Array.isArray(o.assets)) return false;
  return true;
}

// "1" sentinel yields the bundled fixture; otherwise parse as JSON GithubRelease.
function readMockPayload(): GithubRelease | null {
  const raw = readStorage(MOCK_KEY);
  if (!raw) return null;
  if (raw === "1") return MOCK_FIXTURE;
  try {
    const parsed: unknown = JSON.parse(raw);
    return isGithubRelease(parsed) ? parsed : MOCK_FIXTURE;
  } catch {
    return MOCK_FIXTURE;
  }
}

export interface UpdateInfo {
  current: string;
  latest: string;
  hasUpdate: boolean;
  releaseUrl?: string;
  releaseName?: string;
  body?: string;
  publishedAt?: string;
  assetName?: string;
  assetSize?: number;
  assetSha?: string;
  assetUrl?: string;
}

export type ChangelogTag = "new" | "improved" | "balance" | "fixes" | "other";

export interface ChangelogSection {
  tag: ChangelogTag;
  title: string;
  items: string[];
}

export class UpdateCheckError extends Error {
  readonly cause?: unknown;
  constructor(message: string, cause?: unknown) {
    super(message);
    this.name = "UpdateCheckError";
    this.cause = cause;
  }
}

export async function checkForUpdate(
  signal?: AbortSignal,
): Promise<UpdateInfo> {
  const mock = readMockPayload();
  if (mock) {
    await new Promise<void>((resolve, reject) => {
      const t = window.setTimeout(resolve, 300);
      signal?.addEventListener("abort", () => {
        window.clearTimeout(t);
        reject(new DOMException("Aborted", "AbortError"));
      });
    });
    return mapReleaseToUpdateInfo(mock);
  }
  if (!GITHUB_REPO) {
    throw new UpdateCheckError(
      "Update check disabled: set GITHUB_REPO in src/utils/version.ts",
    );
  }
  let res: Response;
  try {
    res = await fetch(
      `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`,
      {
        headers: { Accept: "application/vnd.github+json" },
        signal,
      },
    );
  } catch (err) {
    throw new UpdateCheckError("Network error", err);
  }
  if (!res.ok) {
    throw new UpdateCheckError(`GitHub API ${res.status}`);
  }
  let payload: GithubRelease;
  try {
    const raw: unknown = await res.json();
    if (!isGithubRelease(raw)) {
      throw new UpdateCheckError("Invalid response shape");
    }
    payload = raw;
  } catch (err) {
    if (err instanceof UpdateCheckError) throw err;
    throw new UpdateCheckError("Invalid response", err);
  }
  return mapReleaseToUpdateInfo(payload);
}

interface GithubAsset {
  name?: string;
  size?: number;
  digest?: string;
  browser_download_url?: string;
}

interface GithubRelease {
  tag_name?: string;
  html_url?: string;
  name?: string;
  body?: string;
  published_at?: string;
  assets?: GithubAsset[];
}

export function mapReleaseToUpdateInfo(payload: GithubRelease): UpdateInfo {
  const rawTag = payload.tag_name?.trim() ?? "";
  if (!rawTag) {
    throw new UpdateCheckError("No tag in latest release");
  }
  const latest = rawTag.replace(/^v/, "");
  const asset = pickAsset(payload.assets);
  return {
    current: APP_VERSION,
    latest,
    hasUpdate: compareSemver(latest, APP_VERSION) > 0,
    releaseUrl: payload.html_url,
    releaseName: payload.name,
    body: payload.body,
    publishedAt: payload.published_at,
    assetName: asset?.name,
    assetSize: asset?.size,
    assetSha: asset?.digest?.replace(/^sha256:/i, ""),
    assetUrl: asset?.browser_download_url,
  };
}

// Heuristic: drop sig/checksum/source files, pick the largest remaining binary as "the real installer".
function pickAsset(assets?: GithubAsset[]): GithubAsset | undefined {
  if (!assets || assets.length === 0) return undefined;
  const candidates = assets.filter((a) => {
    const name = (a.name ?? "").toLowerCase();
    if (!name) return false;
    if (/\.(asc|sig|sha256|sha512|txt)$/.test(name)) return false;
    if (/source/.test(name)) return false;
    return true;
  });
  if (candidates.length === 0) return undefined;
  return candidates.reduce((best, cur) =>
    (cur.size ?? 0) > (best.size ?? 0) ? cur : best,
  );
}

const TAG_ALIASES: Record<string, ChangelogTag> = {
  new: "new",
  added: "new",
  features: "new",
  feature: "new",
  adds: "new",
  improved: "improved",
  improvements: "improved",
  changed: "improved",
  changes: "improved",
  updates: "improved",
  enhancements: "improved",
  balance: "balance",
  tweaks: "balance",
  fixes: "fixes",
  fixed: "fixes",
  bugs: "fixes",
  bugfix: "fixes",
  bugfixes: "fixes",
  patches: "fixes",
};

export function parseChangelog(body: string): ChangelogSection[] {
  const lines = body.replace(/\r\n/g, "\n").split("\n");
  const sections: ChangelogSection[] = [];
  let current: ChangelogSection | null = null;

  const headingRe = /^#{1,3}\s+(.+?)\s*$/;
  const bulletRe = /^\s*[-*]\s+(.+?)\s*$/;

  for (const raw of lines) {
    const headMatch = headingRe.exec(raw);
    if (headMatch) {
      const title = headMatch[1]!.trim();
      const tag = matchTag(title);
      current = { tag, title, items: [] };
      sections.push(current);
      continue;
    }
    const bulletMatch = bulletRe.exec(raw);
    if (bulletMatch) {
      const item = bulletMatch[1]!.trim();
      if (!current) {
        current = { tag: "other", title: "", items: [] };
        sections.push(current);
      }
      current.items.push(item);
    }
  }
  return sections.filter((s) => s.items.length > 0);
}

function matchTag(title: string): ChangelogTag {
  const key = title.toLowerCase().replace(/[^a-z]/g, "");
  return TAG_ALIASES[key] ?? "other";
}

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const decimals = i === 0 || v >= 100 ? 0 : v >= 10 ? 1 : 2;
  return `${v.toFixed(decimals)} ${units[i]}`;
}

export function shortSha(sha: string, head = 4, tail = 4): string {
  const clean = sha.replace(/^sha256:/i, "");
  if (clean.length <= head + tail + 1) return clean;
  return `${clean.slice(0, head)}…${clean.slice(-tail)}`;
}

// Ignores pre-release suffix.
export function compareSemver(a: string, b: string): number {
  const parse = (v: string) =>
    v
      .split("-")[0]!
      .split(".")
      .map((part) => {
        const n = Number.parseInt(part, 10);
        return Number.isFinite(n) ? n : 0;
      });
  const pa = parse(a);
  const pb = parse(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const na = pa[i] ?? 0;
    const nb = pb[i] ?? 0;
    if (na > nb) return 1;
    if (na < nb) return -1;
  }
  return 0;
}
