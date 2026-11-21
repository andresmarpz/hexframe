import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import sitemap from "@astrojs/sitemap";

export default defineConfig({
  site: process.env.SITE_URL ?? "https://greyscreen-lake.vercel.app",
  output: "static",
  integrations: [react(), sitemap()],
  build: { inlineStylesheets: "auto" },
});
