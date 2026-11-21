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
    mkdir -p dist
    rm -f dist/greyscreen-windows-x64.zip
    zip -j dist/greyscreen-windows-x64.zip target/{{win}}/release/greyscreen.exe target/{{win}}/release/greyscreen-cli.exe packaging/README.txt

ci: lint check-win test
