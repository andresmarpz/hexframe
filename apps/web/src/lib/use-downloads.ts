import { useEffect, useState } from "react";
import { repo } from "../content/copy";
import { cachedFetch } from "./github";
import { mergeDownloads, pickDownloads, type Downloads } from "./release";

export function useDownloads(initial: Downloads | null): Downloads | null {
  const [downloads, setDownloads] = useState(initial);

  useEffect(() => {
    let live = true;
    cachedFetch("greyscreen:release:v2", `https://api.github.com/repos/${repo}/releases?per_page=30`, pickDownloads).then(
      (fetched) => {
        if (live) setDownloads((current) => mergeDownloads(current, fetched));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  return downloads;
}
