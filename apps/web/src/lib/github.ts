const TTL_MS = 10 * 60 * 1000;

interface Entry<T> {
  at: number;
  value: T;
}

export async function cachedFetch<T>(
  key: string,
  url: string,
  pick: (json: unknown) => T,
): Promise<T> {
  try {
    const raw = sessionStorage.getItem(key);
    if (raw) {
      const entry = JSON.parse(raw) as Entry<T>;
      if (Date.now() - entry.at < TTL_MS) return entry.value;
    }
  } catch {
  }
  let value: T;
  try {
    const res = await fetch(url, { headers: { Accept: "application/vnd.github+json" } });
    value = res.ok ? pick(await res.json()) : pick(null);
  } catch {
    value = pick(null);
  }
  try {
    sessionStorage.setItem(key, JSON.stringify({ at: Date.now(), value } satisfies Entry<T>));
  } catch {
  }
  return value;
}
