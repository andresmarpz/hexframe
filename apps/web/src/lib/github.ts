const TTL_MS = 10 * 60 * 1000;

interface Entry<T> {
  at: number;
  value: T;
}

const inflight = new Map<string, Promise<unknown>>();

function readCache<T>(key: string): T | undefined {
  try {
    const raw = sessionStorage.getItem(key);
    if (!raw) return undefined;
    const entry = JSON.parse(raw) as Entry<T>;
    return Date.now() - entry.at < TTL_MS ? entry.value : undefined;
  } catch {
    return undefined;
  }
}

function writeCache<T>(key: string, value: T): void {
  try {
    sessionStorage.setItem(key, JSON.stringify({ at: Date.now(), value } satisfies Entry<T>));
  } catch {
  }
}

async function load<T>(key: string, url: string, pick: (json: unknown) => T | null): Promise<T | null> {
  try {
    const res = await fetch(url, { headers: { Accept: "application/vnd.github+json" } });
    if (!res.ok) return null;
    const value = pick(await res.json());
    if (value !== null) writeCache(key, value);
    return value;
  } catch {
    return null;
  }
}

export function cachedFetch<T>(key: string, url: string, pick: (json: unknown) => T | null): Promise<T | null> {
  const cached = readCache<T>(key);
  if (cached !== undefined) return Promise.resolve(cached);
  let pending = inflight.get(key) as Promise<T | null> | undefined;
  if (!pending) {
    pending = load(key, url, pick).finally(() => inflight.delete(key));
    inflight.set(key, pending);
  }
  return pending;
}
