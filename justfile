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

# Builds, checks and publishes a GitHub release from this Mac. It uses no Actions minutes.
# A tag with a dash (v0.2.0-preview.1) becomes a prerelease.
release tag notes="": ci dist
    git diff --quiet && git diff --cached --quiet || { echo "commit your changes first"; exit 1; }
    git fetch -q origin && test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" || { echo "push main first"; exit 1; }
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); case "{{tag}}" in v$version*) ;; *) echo "tag {{tag}} does not match version $version"; exit 1;; esac
    GH_TOKEN=$(gh auth token --user andresmarpz) gh release create "{{tag}}" dist/greyscreen.exe dist/greyscreen-cli.exe dist/README.txt -R andresmarpz/greyscreen --target main --title "{{tag}}" --notes "{{notes}}" {{ if tag =~ '-' { "--prerelease" } else { "" } }}
