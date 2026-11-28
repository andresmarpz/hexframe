import { repo } from "../content/copy";
import { pickDownloads, type Downloads } from "./release";

const cache = new Map<string, Promise<unknown>>();

function fetchJson(path: string): Promise<unknown> {
  let pending = cache.get(path);
  if (!pending) {
    const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN;
    const headers: Record<string, string> = {
      Accept: "application/vnd.github+json",
      "User-Agent": "greyscreen-web-build",
    };
    if (token) headers.Authorization = `Bearer ${token}`;
    pending = fetch(`https://api.github.com/repos/${repo}${path}`, { headers })
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      })
      .catch((error: unknown) => {
        console.warn(`github ${path || "/"}: ${error instanceof Error ? error.message : String(error)}`);
        return null;
      });
    cache.set(path, pending);
  }
  return pending;
}

export async function getDownloads(): Promise<Downloads | null> {
  return pickDownloads(await fetchJson("/releases?per_page=30"));
}
