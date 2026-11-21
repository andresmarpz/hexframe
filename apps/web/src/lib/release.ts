export const APP_ASSET = "greyscreen.exe";
export const CLI_ASSET = "greyscreen-cli.exe";

export interface Asset {
  url: string;
  size: number;
}

export interface Downloads {
  tag: string;
  date: string;
  prerelease: boolean;
  pageUrl: string;
  app: Asset | null;
  cli: Asset | null;
}

interface ApiAsset {
  name?: unknown;
  size?: unknown;
  browser_download_url?: unknown;
}

interface ApiRelease {
  tag_name?: unknown;
  published_at?: unknown;
  prerelease?: unknown;
  draft?: unknown;
  html_url?: unknown;
  assets?: unknown;
}

function findAsset(assets: ApiAsset[], name: string): Asset | null {
  const asset = assets.find((a) => a.name === name);
  if (typeof asset?.browser_download_url !== "string" || typeof asset.size !== "number") return null;
  return { url: asset.browser_download_url, size: asset.size };
}

function toDownloads(raw: ApiRelease): Downloads | null {
  if (raw.draft === true) return null;
  if (typeof raw.tag_name !== "string" || typeof raw.published_at !== "string" || typeof raw.html_url !== "string") {
    return null;
  }
  const assets = Array.isArray(raw.assets) ? (raw.assets as ApiAsset[]) : [];
  return {
    tag: raw.tag_name,
    date: raw.published_at,
    prerelease: raw.prerelease === true,
    pageUrl: raw.html_url,
    app: findAsset(assets, APP_ASSET),
    cli: findAsset(assets, CLI_ASSET),
  };
}

export function pickDownloads(json: unknown): Downloads | null {
  if (!Array.isArray(json)) return null;
  const releases = (json as ApiRelease[])
    .map(toDownloads)
    .filter((r): r is Downloads => r !== null)
    .sort((a, b) => Date.parse(b.date) - Date.parse(a.date));
  return releases.find((r) => r.app) ?? releases[0] ?? null;
}

export function mergeDownloads(current: Downloads | null, fetched: Downloads | null): Downloads | null {
  if (!fetched) return current;
  if (current?.app && !fetched.app) return current;
  return fetched;
}

export function pickStars(json: unknown): number | null {
  const count = (json as { stargazers_count?: unknown } | null)?.stargazers_count;
  return typeof count === "number" ? count : null;
}

export function mergeStars(current: number | null, fetched: number | null): number | null {
  return fetched ?? current;
}
