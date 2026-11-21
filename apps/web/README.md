# Greyscreen website

Static marketing site: Astro, React islands for the GitHub star count and the download links, plain CSS.

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

The build reads the repo's releases and star count from the GitHub API (`src/lib/github-build.ts`), so the static HTML already has the newest release that carries `greyscreen.exe`, with its version, date and size. If no release has it, the buttons link to the newest release page, then to `/releases`. The islands refresh the same data in the browser (sessionStorage, 10 minutes) and never replace good data with worse. The `just web-*` build recipes pass a token from `gh` for the `andresmarpz` account: unauthenticated builds hit the 60 requests per hour limit. `just release` redeploys the site, which refreshes the links.

## Where things live

- `src/content/copy.ts`: all page copy. Strings marked `TODO(copy)` are not in the written copy yet.
- `src/assets/brand/logo-mark.svg` and `public/favicon.svg`: logo and favicon. Replace the files to rebrand.
- `src/styles/rune-tokens.css`: the app's colours, type sizes and radii. Generated, do not edit.
- `src/styles/global.css`: site styles on top of those tokens.
- `src/content/changelog/<version>.md`: one file per release (`version`, `date`, `title`, then bullets).
- `public/screens/*.png`: app screenshots.

## Refresh the theme tokens

```
just web-tokens
```

Runs `crates/app/examples/export_tokens.rs`, which prints `rune_kit::Kit::neutral` (dark, and light under `prefers-color-scheme`) as CSS custom properties.

## Refresh the screenshots

```
just web-screens
```

Builds the app and renders the scenes with `greyscreen --snap`. The recipe sets `HOME=/Users/Shared`, so the Recordings page shows a neutral folder path and not the real user name. Check each PNG for private paths before committing.

## Not in CI

`just ci` does not touch the website.
