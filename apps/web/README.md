# Greyscreen website

Static site (home, download, FAQ, privacy, changelog): Next.js App Router with `output: "export"`, React 19, TypeScript 7, CSS modules on Paper tokens. Light theme only. The design lives in the Paper file "Greyscreen App" and `docs/design/website.md`.

## Run

From the repo root:

```
bun install
just web-dev      # dev server on http://localhost:3100 (PORT overrides)
just web-check    # typecheck, then a production build
just web-build    # static export into apps/web/out
```

In `apps/web`, `bun run typecheck` runs `next typegen` and `tsc --noEmit` (TypeScript 7). `bun run start` serves `out/`. Set `SITE_URL` to the public address before building, so canonical links and the sitemap are right.

## GitHub data

The build reads the repo's releases and star count from the GitHub API (`src/lib/github-build.ts`), so the static HTML already has the newest release that carries `GreyscreenSetup.exe` (or the older `GreyscreenSetup.exe` or `greyscreen.exe`), with its version, date and size. If no release has it, the buttons link to the newest release page, then to `/releases`. The client components `DownloadButton`, `ReleaseInfo` and `GitHubButton` (`src/components/site/`) refresh the same data in the browser (sessionStorage, 10 minutes) and never replace good data with worse. The `just web-*` recipes pass a token from `gh` for the `andresmarpz` account: unauthenticated builds hit the 60 requests per hour limit. Vercel's Git builds carry no token, so they can hit that limit and fall back to the releases link until the next deploy. `just release` redeploys the site with `just web-deploy` (a local, authenticated build), which refreshes the links; a release is not a commit, so a push cannot do it.

## Where things live

- `src/app/`: routes, layout (fonts, metadata, analytics), sitemap and robots.
- `src/components/site/`: shared pieces from Paper (`Mark`, `Lockup`, `Nav`, `Footer`, `RespawnLabel`, `Callout`, `RecDot`, `XBullet`, `Stage`, `Section`, `TextLink`, `DownloadButton`, `ReleaseInfo`, `GitHubButton`).
- `src/components/<page>/`: one file per page section (`home`, `download`, `faq`, `changelog`, `privacy`, `notfound`).
- `src/components/kit/`: the Rune kit port (`Button`, `Badge`, `Keybind`) on the light tokens. `xxl` is a website-only Button size.
- `src/styles/tokens.css`: every Paper token the site uses, as CSS custom properties. `src/styles/global.css`: base styles and type classes. Fonts come from `next/font/google` in `layout.tsx` (`--font-sans`, `--font-mono`).
- `src/content/copy.ts`: site metadata, nav and footer links, page copy. `discordUrl` is empty: the Discord links stay hidden until you set it.
- `src/content/faq.ts` and `src/content/privacy.ts`: the `/faq/` and `/privacy/` pages. Keep `privacy.ts` true to the code: update checks, Data Dragon icons, Send feedback payload.
- `src/content/changelog/<version>.md`: one file per release (`version`, `date`, `title`, then bullets). `just release` and `scripts/release-meta.sh` read these. `src/lib/changelog.ts` loads them at build time.
- `public/`: favicons, `og.png`, `screens/` (app screens exported from Paper).

## Help anchors

The app's "Learn more" links open `https://greyscreen.app/faq#<anchor>`. Each anchor is the `id` of an item in `src/content/faq.ts`. Do not rename them:

`vanguard`, `fps`, `storage`, `low-disk`, `death-review`, `fullscreen`, `window-not-found`, `run-as-admin`, `folder-not-writable`, `no-encoder`, `no-gpu`, `video-conversion`, `updates`, `uninstall`, `privacy`, `free`, `help`.

## Analytics

`@vercel/analytics` (cookie-less page views) is in `src/app/layout.tsx`. Turn Web Analytics on once in the Vercel project (Analytics tab) or it records nothing.

## Refresh the screenshots

```
just web-screens
```

Builds the app and renders the scenes with `greyscreen --snap`. The recipe sets `HOME=/Users/Shared`, so any folder path on screen is neutral and not the real user name. Check each PNG for private paths before committing.

## Deploy

Vercel project `greyscreen` (team `andresmarpzs-projects`) is connected to `andresmarpz/greyscreen` and publishes https://greyscreen.app (DNS at Namecheap: two A records for `@` and a CNAME for `www`, which redirects to the bare domain). The old https://greyscreen-lake.vercel.app redirects there with the path kept, so links in older app versions still work.

- Vercel's Git deployments are off (`git.deploymentEnabled` is false in `apps/web/vercel.json`), so no push creates a deployment by itself. The workflow `.github/workflows/web.yml` runs on pushes to `main` that change `apps/web/`, `brand/`, `package.json` or `bun.lock`, and triggers a production deploy through the Vercel deploy hook stored in the `VERCEL_DEPLOY_HOOK` repo secret. Rust-only pushes create nothing on Vercel. The workflow can also be run by hand.
- Project settings (in Vercel, not in the repo): root directory `apps/web`, "include source files outside the root directory" on, Node 24.x. The framework, install and build commands live in `apps/web/vercel.json`. The framework preset in the dashboard must not override it, and the output directory setting stays empty: the Next.js runtime reads `.next` and writes `.vercel/output`.
- `SITE_URL` is set for production only. It is a sensitive variable, so `vercel pull` returns a placeholder; `src/lib/site-url.ts` falls back to the production URL when the value is not a URL.
- `just web-deploy` is the manual path: a local build with a GitHub token, uploaded as a prebuilt production deployment, then a check of the main URLs.

## Not in CI

`just check` does not touch the website.
