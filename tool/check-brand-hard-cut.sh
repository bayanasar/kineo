#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
legacy="$(printf '%s%s' 'kin' 'eo')"

paths="$(git -C "$root" ls-files | grep -i -- "$legacy" || true)"
contents="$(git -C "$root" grep -n -I -i -- "$legacy" -- . || true)"

if [[ -n "$paths" || -n "$contents" ]]; then
  echo "legacy product name detected in tracked source" >&2
  if [[ -n "$paths" ]]; then
    echo "tracked paths:" >&2
    printf '%s\n' "$paths" >&2
  fi
  if [[ -n "$contents" ]]; then
    echo "tracked contents:" >&2
    printf '%s\n' "$contents" >&2
  fi
  exit 1
fi

echo "Kineto brand hard-cut check passed"
