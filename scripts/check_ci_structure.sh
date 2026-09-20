#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

failures=0

check() {
  local description="$1"
  shift
  if "$@"; then
    printf 'ok: %s\n' "$description"
  else
    printf 'error: %s\n' "$description" >&2
    failures=$((failures + 1))
  fi
}

check "GitHub Actions restores the Rust build cache" \
  grep -Eq 'uses: +Swatinem/rust-cache@' .github/workflows/ci.yml
check "GitHub Actions caches the C++-heavy workspace crate" \
  grep -Eq 'cache-workspace-crates: +true' .github/workflows/ci.yml
check "all Cargo phases inherit one RUSTFLAGS value" \
  grep -Eq '^export RUSTFLAGS *\?=' Makefile
check "the test recipe does not override RUSTFLAGS" \
  bash -c '! grep -Eq '\''^[[:space:]]*RUSTFLAGS=.*cargo test'\'' Makefile'
check "the validation suite has no redundant build prerequisite" \
  bash -c '! grep -Eq '\''^check-suite:.*(^|[[:space:]])build([[:space:]]|$)'\'' Makefile'
check "the product exposes a library crate" test -f src/lib.rs
check "integration tests import the library instead of embedding src modules" \
  bash -c '! grep -R -Eq '\''#\[path = "\.\./src/'\'' tests --include='\''*.rs'\'''
check "CXX-Qt watches only the exported header tree" \
  grep -Fq '.crate_include_root(Some("cpp".to_owned()))' build.rs

if ((failures > 0)); then
  printf '%d CI structure check(s) failed\n' "$failures" >&2
  exit 1
fi
