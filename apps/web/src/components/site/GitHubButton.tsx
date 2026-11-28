"use client";

import { useEffect, useState } from "react";
import { githubUrl, repo } from "../../content/copy";
import { cachedFetch } from "../../lib/github";
import { mergeStars, pickStars } from "../../lib/release";
import { Button, type ButtonSize } from "../kit/Button";
import { GitHubIcon } from "./icons";

interface Props {
  initialStars: number | null;
  size?: ButtonSize;
  className?: string;
}

function formatStars(count: number): string {
  if (count < 1000) return String(count);
  return `${(count / 1000).toFixed(count < 10000 ? 1 : 0).replace(/\.0$/, "")}k`;
}

export function GitHubButton({ initialStars, size = "xxl", className }: Props) {
  const [stars, setStars] = useState(initialStars);

  useEffect(() => {
    let live = true;
    cachedFetch("greyscreen:stars", `https://api.github.com/repos/${repo}`, pickStars).then((count) => {
      if (live) setStars((current) => mergeStars(current, count));
    });
    return () => {
      live = false;
    };
  }, []);

  return (
    <Button
      href={githubUrl}
      variant="outline"
      size={size}
      className={className}
      icon={<GitHubIcon size={size === "xxl" ? 18 : 16} />}
      aria-label={stars ? `Greyscreen on GitHub, ${stars} ${stars === 1 ? "star" : "stars"}` : "Greyscreen on GitHub"}
    >
      View on GitHub{stars ? ` · ${formatStars(stars)}` : ""}
    </Button>
  );
}
