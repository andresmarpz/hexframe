set shell := ["bash", "-euo", "pipefail", "-c"]

win := "x86_64-pc-windows-msvc"

export RUSTC_WRAPPER := env("RUSTC_WRAPPER", `command -v sccache || true`)

default:
    @just --list

fmt:
    cargo fmt --all

# Everyday gate, run before every push: fmt and clippy. No tests.
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings

spell:
    typos

# Runs the tests matching a filter, for example `just test mp4`.
test *filter:
    cargo nextest run --workspace --locked {{filter}}

build-win:
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"; cargo xwin build --release --locked --target {{win}} -p greyscreen -p greyscreen-cli
    @ls -lh target/{{win}}/release/greyscreen.exe target/{{win}}/release/greyscreen-cli.exe

# greyscreen.exe is a copy of greyscreen.exe: installed Greyscreen builds update by downloading that asset name.
dist: build-win
    rm -rf dist
    mkdir -p dist
    cp target/{{win}}/release/greyscreen.exe target/{{win}}/release/greyscreen-cli.exe packaging/README.txt dist/
    cp LICENSE dist/LICENSE.txt
    cp dist/greyscreen.exe dist/GreyscreenSetup.exe
    cp dist/greyscreen.exe dist/greyscreen.exe

# Builds an ad-hoc signed Greyscreen.app for UI testing on this Mac and installs it in ~/Applications. Never published.
mac-app:
    packaging/macos/build-app.sh

# Same bundle without installing it.
mac-bundle:
    packaging/macos/build-app.sh --no-install

# Regenerates every raster from the SVG sources in brand/.
brand:
    mkdir -p apps/web/public
    rsvg-convert -w 512 -h 512 brand/app-icon.svg -o crates/app/assets/icon.png
    d=$(mktemp -d); \
    rsvg-convert -w 16 -h 16 brand/app-icon-16.svg -o $d/16.png; \
    for n in 24 32 48 64 128 256; do rsvg-convert -w $n -h $n brand/app-icon.svg -o $d/$n.png; done; \
    magick $d/16.png $d/24.png $d/32.png $d/48.png $d/64.png $d/128.png $d/256.png crates/app/assets/greyscreen.ico; \
    rm -rf $d
    rsvg-convert -w 32 -h 32 brand/tray-icon.svg -o crates/app/assets/tray.png
    rsvg-convert -w 32 -h 32 brand/tray-recording.svg -o crates/app/assets/tray-recording.png
    cp brand/favicon.svg apps/web/public/favicon.svg
    d=$(mktemp -d); \
    rsvg-convert -w 32 -h 32 brand/favicon.svg -o $d/32.png; \
    rsvg-convert -w 16 -h 16 brand/app-icon-16.svg -o $d/16.png; \
    magick $d/32.png $d/16.png apps/web/public/favicon.ico; \
    rm -rf $d
    rsvg-convert -w 180 -h 180 brand/app-icon.svg -o apps/web/public/apple-touch-icon.png
    rsvg-convert -w 1200 -h 630 brand/og-image.svg -o apps/web/public/og.png
    sed 's|viewBox="0 0 1200 630"|viewBox="0 15 1200 600"|' brand/og-image.svg | rsvg-convert -w 1280 -h 640 -o brand/social-preview.png

web-tokens:
    cargo run -q --locked -p greyscreen --example export_tokens > apps/web/src/styles/rune-tokens.css

# HOME points at /Users/Shared so the Recordings subtitle shows a neutral path, not the real user.
web-screens:
    cargo build -q --locked -p greyscreen
    mkdir -p apps/web/public/screens "/Users/Shared/Library/Application Support/Greyscreen"
    ln -sfn "$HOME/Library/Application Support/Greyscreen/cache" "/Users/Shared/Library/Application Support/Greyscreen/cache"
    for scene in recording last-game recordings settings; do HOME=/Users/Shared target/debug/greyscreen --snap apps/web/public/screens/$scene.png --scene $scene --scale 2 --theme dark || echo "skipped $scene"; done
    rm "/Users/Shared/Library/Application Support/Greyscreen/cache"
    rmdir "/Users/Shared/Library/Application Support/Greyscreen" "/Users/Shared/Library/Application Support" /Users/Shared/Library

web-dev:
    cd apps/web && bun run dev

web-build:
    export GH_TOKEN="${GH_TOKEN:-$(gh auth token --user andresmarpz 2>/dev/null)}"; cd apps/web && bun run build

web-check:
    export GH_TOKEN="${GH_TOKEN:-$(gh auth token --user andresmarpz 2>/dev/null)}"; cd apps/web && bun run check && bun run build

api-dev:
    cd apps/api && bun run dev

api-test:
    cd apps/api && bun run test

api-check:
    cd apps/api && bun run typecheck && bun run test

# Manual deploy to https://greyscreen.app with a local build that carries a GitHub token. Pushes to main that touch the site deploy through the web.yml workflow and a Vercel deploy hook; this recipe is the manual fallback.
web-deploy:
    export VERCEL_ORG_ID=team_jHrXpSIdA6RnJeTxQQd0q1pP VERCEL_PROJECT_ID=prj_4SHas5qPeNUS4xqwKSnucDo2Zr95 GH_TOKEN="${GH_TOKEN:-$(gh auth token --user andresmarpz 2>/dev/null)}"; vercel pull --yes --environment=production && vercel build --prod --yes
    test -s .vercel/output/static/index.html || { echo "web-deploy: .vercel/output/static/index.html is missing, not deploying" >&2; exit 1; }
    VERCEL_ORG_ID=team_jHrXpSIdA6RnJeTxQQd0q1pP VERCEL_PROJECT_ID=prj_4SHas5qPeNUS4xqwKSnucDo2Zr95 vercel deploy --prebuilt --prod --yes
    for path in / /download /changelog /og.png /favicon.svg; do code=$(curl -s -o /dev/null -w '%{http_code}' "https://greyscreen.app$path"); echo "$code $path"; [ "$code" = 200 ] || { echo "web-deploy: $path returned $code after deploy" >&2; exit 1; }; done

# Title and notes come from apps/web/src/content/changelog/<version>.md. A tag with a dash is a prerelease.
# The tree must be clean and its commit already on origin/main. The release tags that commit, so later pushes to main do not stop it.
# Builds, checks and publishes a GitHub release from this Mac. It uses no Actions minutes.
release tag:
    git diff --quiet && git diff --cached --quiet || { echo "commit your changes first"; exit 1; }
    git fetch -q origin && git merge-base --is-ancestor HEAD origin/main || { echo "push main first"; exit 1; }
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); test "{{tag}}" = "v$version" || { echo "tag {{tag}} must equal v$version: set the workspace version to the full release version first"; exit 1; }
    just release-publish {{tag}} "$(git rev-parse HEAD)"

# The gate and the Windows build run side by side.
[parallel, private]
release-publish tag sha: check dist
    test "$(git rev-parse HEAD)" = "{{sha}}" || { echo "HEAD moved during the build, release not published"; exit 1; }
    scripts/release-meta.sh notes {{tag}} > dist/notes.md
    GH_TOKEN=$(gh auth token --user andresmarpz) gh release create "{{tag}}" dist/GreyscreenSetup.exe dist/greyscreen.exe dist/greyscreen.exe dist/greyscreen-cli.exe dist/README.txt dist/LICENSE.txt -R andresmarpz/greyscreen --target {{sha}} --title "$(scripts/release-meta.sh title {{tag}})" --notes-file dist/notes.md {{ if tag =~ '-' { "--prerelease" } else { "--latest" } }}
    just web-deploy

# Rewrites the title and notes of every published release from its changelog entry.
release-sync:
    for entry in apps/web/src/content/changelog/*.md; do v=$(basename "$entry" .md); GH_TOKEN=$(gh auth token --user andresmarpz) gh release edit "v$v" -R andresmarpz/greyscreen --title "$(scripts/release-meta.sh title "$v")" --notes "$(scripts/release-meta.sh notes "$v")"; done
