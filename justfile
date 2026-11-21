set shell := ["bash", "-euo", "pipefail", "-c"]

win := "x86_64-pc-windows-msvc"

export RUSTC_WRAPPER := env("RUSTC_WRAPPER", `command -v sccache || true`)

default:
    @just --list

fmt:
    cargo fmt --all

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    typos

check-win:
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"; cargo xwin clippy --workspace --all-targets --target {{win}} --locked -- -D warnings

test:
    cargo test --workspace --locked

build-win:
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"; cargo xwin build --release --locked --target {{win}} -p greyscreen -p greyscreen-cli
    @ls -lh target/{{win}}/release/greyscreen.exe target/{{win}}/release/greyscreen-cli.exe

dist: build-win
    rm -rf dist
    mkdir -p dist
    cp target/{{win}}/release/greyscreen.exe target/{{win}}/release/greyscreen-cli.exe packaging/README.txt dist/

ci: lint check-win test

web-tokens:
    cargo run -q --locked -p greyscreen --example export_tokens > apps/web/src/styles/rune-tokens.css

# HOME points at /Users/Shared so the Recordings subtitle shows a neutral path, not the real user.
web-screens:
    cargo build -q --locked -p greyscreen
    mkdir -p apps/web/public/screens "/Users/Shared/Library/Application Support/Greyscreen"
    ln -sfn "$HOME/Library/Application Support/Greyscreen/cache" "/Users/Shared/Library/Application Support/Greyscreen/cache"
    for scene in recording last-game recordings review review-theater settings; do HOME=/Users/Shared target/debug/greyscreen --snap apps/web/public/screens/$scene.png --scene $scene --scale 2 --theme dark || echo "skipped $scene"; done
    rm "/Users/Shared/Library/Application Support/Greyscreen/cache"
    rmdir "/Users/Shared/Library/Application Support/Greyscreen" "/Users/Shared/Library/Application Support" /Users/Shared/Library

web-dev:
    cd apps/web && bun run dev

web-build:
    cd apps/web && bun run build

web-check:
    cd apps/web && bun run check && bun run build

# Builds the site and publishes it to Vercel production at https://greyscreen-lake.vercel.app.
web-deploy:
    cd apps/web && SITE_URL=https://greyscreen-lake.vercel.app bun run build
    VERCEL_ORG_ID=team_jHrXpSIdA6RnJeTxQQd0q1pP VERCEL_PROJECT_ID=prj_4SHas5qPeNUS4xqwKSnucDo2Zr95 vercel deploy apps/web/dist --prod --yes

# Builds, checks and publishes a GitHub release from this Mac. It uses no Actions minutes.
# Title and notes come from apps/web/src/content/changelog/<version>.md. A tag with a dash is a prerelease.
release tag: ci dist
    git diff --quiet && git diff --cached --quiet || { echo "commit your changes first"; exit 1; }
    git fetch -q origin && test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" || { echo "push main first"; exit 1; }
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); case "{{tag}}" in v$version*) ;; *) echo "tag {{tag}} does not match version $version"; exit 1;; esac
    scripts/release-meta.sh notes {{tag}} > dist/notes.md
    GH_TOKEN=$(gh auth token --user andresmarpz) gh release create "{{tag}}" dist/greyscreen.exe dist/greyscreen-cli.exe dist/README.txt -R andresmarpz/greyscreen --target main --title "$(scripts/release-meta.sh title {{tag}})" --notes-file dist/notes.md {{ if tag =~ '-' { "--prerelease" } else { "--latest" } }}
    just web-deploy

# Rewrites the title and notes of every published release from its changelog entry.
release-sync:
    for entry in apps/web/src/content/changelog/*.md; do v=$(basename "$entry" .md); GH_TOKEN=$(gh auth token --user andresmarpz) gh release edit "v$v" -R andresmarpz/greyscreen --title "$(scripts/release-meta.sh title "$v")" --notes "$(scripts/release-meta.sh notes "$v")"; done
