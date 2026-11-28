import type { MetadataRoute } from "next";
import { siteUrl } from "../lib/site-url";

export const dynamic = "force-static";

const paths = ["/", "/download/", "/faq/", "/changelog/", "/privacy/", "/code-signing/"];

export default function sitemap(): MetadataRoute.Sitemap {
  return paths.map((path) => ({ url: new URL(path, siteUrl).toString() }));
}
