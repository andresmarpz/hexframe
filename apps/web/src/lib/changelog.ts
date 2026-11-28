import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

export interface ChangelogEntry {
  version: string;
  date: string;
  title: string;
  prerelease: boolean;
  bullets: string[];
}

const directory = path.join(process.cwd(), "src/content/changelog");

function parse(source: string): ChangelogEntry | null {
  const match = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/.exec(source);
  if (!match) return null;
  const fields = new Map<string, string>();
  for (const line of (match[1] ?? "").split("\n")) {
    const at = line.indexOf(":");
    if (at > 0) fields.set(line.slice(0, at).trim(), line.slice(at + 1).trim());
  }
  const version = fields.get("version");
  const date = fields.get("date");
  const title = fields.get("title");
  if (!version || !date || !title) return null;
  const bullets = (match[2] ?? "")
    .split("\n")
    .filter((line) => line.startsWith("- "))
    .map((line) => line.slice(2).trim());
  return { version, date, title, prerelease: version.includes("-"), bullets };
}

export function getChangelog(): ChangelogEntry[] {
  return readdirSync(directory)
    .filter((name) => name.endsWith(".md"))
    .map((name) => parse(readFileSync(path.join(directory, name), "utf8")))
    .filter((entry): entry is ChangelogEntry => entry !== null)
    .sort(
      (a, b) =>
        Date.parse(b.date) - Date.parse(a.date) || b.version.localeCompare(a.version, "en", { numeric: true }),
    );
}
