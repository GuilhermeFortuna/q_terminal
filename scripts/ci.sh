#!/usr/bin/env bash
set -euo pipefail

# Enter the host user ci.slice when available so local CI yields to interactive work.
# No-ops on hosts/runners without systemd-run or the slice (e.g. GitHub Actions).
if [[ "${CI_RESOURCE_CONTROLLED:-0}" != "1" ]]; then
  if command -v systemd-run >/dev/null 2>&1 &&
     systemctl --user status ci.slice >/dev/null 2>&1; then
    exec systemd-run \
      --user --scope --quiet --collect \
      --slice=ci.slice \
      --setenv=CI_RESOURCE_CONTROLLED=1 \
      "$0" "$@"
  fi
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=================================================="
echo "==> Running q_terminal CI Pipeline"
echo "=================================================="

# Locally, cap Cargo build parallelism at half the logical CPUs and lower CPU/IO
# priority so cold builds (fmt/clippy/build/test) yield to interactive work.
# Override with CARGO_BUILD_JOBS=N. On CI runners (CI set) Cargo keeps its defaults.
NICE=()
if [ -z "${CI:-}" ]; then
  CPUS="$(nproc 2>/dev/null || echo 2)"
  DEFAULT_JOBS=$(( CPUS / 2 ))
  (( DEFAULT_JOBS < 1 )) && DEFAULT_JOBS=1
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$DEFAULT_JOBS}"
  echo "Cargo build jobs: ${CARGO_BUILD_JOBS}"
  if command -v nice >/dev/null 2>&1; then
    NICE=(nice -n 10)
    command -v ionice >/dev/null 2>&1 && NICE=(ionice -c 3 "${NICE[@]}")
  fi
fi

"${NICE[@]}" make --no-print-directory check-suite

echo "=================================================="
echo "==> All q_terminal checks passed successfully!"
echo "=================================================="
