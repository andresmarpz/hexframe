import { useEffect, useState } from "react";
import { cachedFetch } from "../lib/github";

interface Props {
  repo: string;
}

interface Release {
  tag: string;
  date: string;
  prerelease: boolean;
  size: number | null;
  url: string;
}

interface ApiAsset {
  name?: unknown;
  size?: unknown;
}

interface ApiRelease {
  tag_name?: unknown;
  published_at?: unknown;
  prerelease?: unknown;
  draft?: unknown;
  html_url?: unknown;
  assets?: unknown;
}

const PREFERRED_ASSETS = ["greyscreen.exe", "greyscreen-windows-x64.zip"];

function toRelease(raw: ApiRelease): Release | null {
  if (typeof raw.tag_name !== "string" || typeof raw.published_at !== "string") return null;
  const assets = Array.isArray(raw.assets) ? (raw.assets as ApiAsset[]) : [];
  const asset = PREFERRED_ASSETS.map((name) => assets.find((a) => a.name === name)).find(Boolean);
  return {
    tag: raw.tag_name,
    date: raw.published_at,
    prerelease: raw.prerelease === true,
    size: typeof asset?.size === "number" ? asset.size : null,
    url: typeof raw.html_url === "string" ? raw.html_url : "",
  };
}

function pick(json: unknown): Release | null {
  if (!Array.isArray(json)) return null;
  const releases = (json as ApiRelease[]).filter((r) => r.draft !== true).map(toRelease);
  return releases.find((r) => r && !r.prerelease) ?? releases.find(Boolean) ?? null;
}

function megabytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function ReleaseInfo({ repo }: Props) {
  const [release, setRelease] = useState<Release | null>(null);

  useEffect(() => {
    let live = true;
    cachedFetch<Release | null>("greyscreen:release", `https://api.github.com/repos/${repo}/releases?per_page=10`, pick).then(
      (value) => {
        if (live) setRelease(value);
      },
    );
    return () => {
      live = false;
    };
  }, [repo]);

  if (!release) return <p className="release-info" aria-hidden="true" />;

  const date = new Date(release.date).toLocaleDateString("en", { year: "numeric", month: "short", day: "numeric" });
  const parts = [release.tag, date, release.size === null ? null : megabytes(release.size)].filter(Boolean);

  return (
    <p className="release-info">
      <span className="release-label">{release.prerelease ? "Latest preview" : "Latest version"}</span>
      {parts.map((part, i) => (
        <span key={i}>{part}</span>
      ))}
      {release.url && (
        <a href={release.url} rel="noopener">
          Release notes
        </a>
      )}
    </p>
  );
}
