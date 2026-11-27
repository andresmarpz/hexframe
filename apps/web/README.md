# Greyscreen website

Static site (home, download, FAQ, privacy, changelog): Astro, React islands for the GitHub star count and the download links, plain CSS.

## Run

From the repo root:

```
bun install
just web-dev      # dev server
just web-check    # astro check, then a production build
just web-build    # build into apps/web/dist
```

`bun run preview` in `apps/web` serves the built site. Set `SITE_URL` to the public address before building, so canonical links and the sitemap are right.

## GitHub data

The build reads the repo's releases and star count from the GitHub API (`src/lib/github-build.ts`), so the static HTML already has the newest release that carries `GreyscreenSetup.exe` (or the older `greyscreen.exe`), with its version, date and size. If no release has it, the buttons link to the newest release page, then to `/releases`. The islands refresh the same data in the browser (sessionStorage, 10 minutes) and never replace good data with worse. The `just web-*` build recipes pass a token from `gh` for the `andresmarpz` account: unauthenticated builds hit the 60 requests per hour limit. Vercel's Git builds carry no token, so they can hit that limit and fall back to the releases link until the next deploy. `just release` redeploys the site with `just web-deploy` (a local, authenticated build), which refreshes the links; a release is not a commit, so Git cannot do it.

## Where things live

- `src/content/copy.ts`: home and download copy, nav and footer links. Strings marked `TODO(copy)` are not in the written copy yet. `discordUrl` is empty: the Discord links in the footer and the FAQ stay hidden until you set it.
- `src/content/faq.ts` and `src/content/privacy.ts`: the `/faq` and `/privacy` pages. Keep `privacy.ts` true to the code: update checks, Data Dragon icons, Send feedback payload.
- `src/components/SmartScreenNotice.astro`: the "Windows may warn you" box on the download page. Delete it, its use in `download.astro` and the `smartScreen*` strings in `copy.ts` once the exe is code-signed.
- `src/assets/brand/logo-mark.svg` and `public/favicon.svg`: logo and favicon. Replace the files to rebrand.
- `src/styles/rune-tokens.css`: the app's colours, type sizes and radii. Generated, do not edit.
- `src/styles/global.css`: site styles on top of those tokens.
- `src/content/changelog/<version>.md`: one file per release (`version`, `date`, `title`, then bullets).
- `public/screens/*.png`: app screenshots.

## Help anchors

The app's "Learn more" links open `https://greyscreen-lake.vercel.app/faq#<anchor>`. Each anchor is the `id` of an item in `src/content/faq.ts`. Do not rename them:

`vanguard`, `fps`, `storage`, `low-disk`, `death-review`, `fullscreen`, `window-not-found`, `run-as-admin`, `folder-not-writable`, `no-encoder`, `no-gpu`, `video-conversion`, `updates`, `uninstall`, `privacy`, `free`, `help`.

## Analytics

`@vercel/analytics` (cookie-less page views) is in `Base.astro`. Turn Web Analytics on once in the Vercel project (Analytics tab) or it records nothing.

## Refresh the theme tokens

```
just web-tokens
```

Runs `crates/app/examples/export_tokens.rs`, which prints `rune_kit::Kit::neutral` (dark, and light under `prefers-color-scheme`) as CSS custom properties.

## Refresh the screenshots

```
just web-screens
```

Builds the app and renders the scenes with `greyscreen --snap`. The recipe sets `HOME=/Users/Shared`, so any folder path on screen is neutral and not the real user name. Check each PNG for private paths before committing.

## Deploy

Vercel project `greyscreen` (team `andresmarpzs-projects`) is connected to `andresmarpz/greyscreen` and publishes https://greyscreen-lake.vercel.app.

- Vercel's Git deployments are off (`git.deploymentEnabled` is false in `apps/web/vercel.json`), so no push creates a deployment by itself. The GitHub workflow `.github/workflows/web.yml` runs on pushes to `main` that change `apps/web/`, `brand/`, `package.json` or `bun.lock`, and triggers a production deploy through the Vercel deploy hook stored in the `VERCEL_DEPLOY_HOOK` repo secret. Rust-only pushes create nothing on Vercel. The workflow can also be run by hand.
- Project settings (in Vercel, not in git): root directory `apps/web`, "include source files outside the root directory" on (the site imports `brand/`), Node 24.x. Install, build and output commands live in `apps/web/vercel.json`.
- `SITE_URL` is set for production only. Preview builds fall back to the production URL in `astro.config.mjs`.
- `just web-deploy` is the manual path: a local build with a GitHub token, uploaded as a prebuilt production deployment, then a check of five URLs.

## Not in CI

`just check` does not touch the website.
