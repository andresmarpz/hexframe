import { useEffect, useState } from "react";
import { cachedFetch } from "../lib/github";
import { mergeStars, pickStars } from "../lib/release";

interface Props {
  repo: string;
  href: string;
  initialStars: number | null;
}

function format(count: number): string {
  if (count < 1000) return String(count);
  return `${(count / 1000).toFixed(count < 10000 ? 1 : 0).replace(/\.0$/, "")}k`;
}

export default function GitHubButton({ repo, href, initialStars }: Props) {
  const [stars, setStars] = useState(initialStars);

  useEffect(() => {
    let live = true;
    cachedFetch("greyscreen:stars", `https://api.github.com/repos/${repo}`, pickStars).then((count) => {
      if (live) setStars((current) => mergeStars(current, count));
    });
    return () => {
      live = false;
    };
  }, [repo]);

  return (
    <a className="gh-button" href={href} aria-label={stars ? `Greyscreen on GitHub, ${stars} ${stars === 1 ? "star" : "stars"}` : "Greyscreen on GitHub, star it"}>
      <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" focusable="false">
        <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.5 7.5 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
      </svg>
      <span>GitHub</span>
      <span className="gh-count">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" focusable="false">
          <path d="M8 .6l2.2 4.7 5.1.7-3.7 3.6.9 5.1L8 12.2l-4.5 2.5.9-5.1L.7 6l5.1-.7L8 .6Z" />
        </svg>
        {stars ? format(stars) : "Star"}
      </span>
    </a>
  );
}
