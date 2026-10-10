#!/bin/sh
# Runs the CI quality gate (.github/workflows/ci.yml) on this machine before you push: format,
# clippy, tests, docs and the repository policy, plus cargo-deny, typos, shellcheck and the commit
# messages of unpushed commits when those tools are installed. Coverage, the minimum Rust version and
# the other operating systems run only in CI. See the CI section of .spec/conventions.md.
#
#   ./scripts/check.sh              # with and without voice input
#   ./scripts/check.sh --no-voice   # only without voice (no CMake or libclang needed)
#   ./scripts/check.sh --gpu        # also lint the gpu feature (Vulkan headers or Metal needed)
set -u

voice=true
gpu=false
for arg in "$@"; do
    case $arg in
        --no-voice) voice=false ;;
        --gpu) gpu=true ;;
        -h | --help)
            sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown option: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

cd "$(dirname "$0")/.." || exit 2

failed=''
# step NAME COMMAND...: runs one check and remembers it when it fails.
step() {
    name=$1
    shift
    printf '\n==> %s\n' "$name"
    if ! "$@"; then
        failed="$failed $name"
    fi
}
skip() {
    printf '\n==> %s: skipped, %s\n' "$1" "$2"
}
have() {
    command -v "$1" >/dev/null 2>&1
}
docs() {
    RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --document-private-items --locked "$@"
}

step format cargo fmt --all -- --check

step clippy-no-default cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
if [ "$voice" = true ]; then
    step clippy-default cargo clippy --workspace --all-targets --locked -- -D warnings
fi
if [ "$gpu" = true ]; then
    step clippy-gpu cargo clippy --workspace --all-targets --locked --features falog-desktop/gpu -- -D warnings
fi

step test-no-default cargo test --workspace --locked --no-default-features
if [ "$voice" = true ]; then
    step test-default cargo test --workspace --locked
    step docs docs
else
    step docs docs --no-default-features
fi

step policy sh scripts/check-policy.sh

if cargo deny --version >/dev/null 2>&1; then
    step dependencies cargo deny --locked check
else
    skip dependencies 'install it with: cargo install --locked cargo-deny'
fi
if cargo machete --version >/dev/null 2>&1; then
    step unused-dependencies cargo machete
else
    skip unused-dependencies 'install it with: cargo install --locked cargo-machete'
fi
if have npx; then
    step duplication npx --yes jscpd@5.4.1 crates tools
else
    skip duplication 'needs Node.js (npx)'
fi
if have typos; then
    step typos typos
else
    skip typos 'install it with: cargo install --locked typos-cli'
fi
if have shellcheck; then
    step shellcheck shellcheck --severity=warning scripts/*.sh
else
    skip shellcheck 'not installed'
fi
if git rev-parse --verify --quiet origin/main >/dev/null; then
    step commits sh scripts/check-commits.sh origin/main..HEAD
else
    skip commits 'no origin/main to compare with'
fi

if [ -n "$failed" ]; then
    printf '\nFailed:%s\n' "$failed" >&2
    exit 1
fi
printf '\nAll checks passed.\n'
