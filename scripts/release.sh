#!/usr/bin/env bash
set -euo pipefail

REPO="mycute-os/mycute"
DIR="${1:-}"

if [[ -z "$DIR" ]]; then
  echo "usage: $0 <directory>" >&2
  exit 1
fi

if [[ ! -d "$DIR" ]]; then
  echo "error: directory not found: $DIR" >&2
  exit 1
fi

DIR="${DIR%/}"
DIR="${DIR%\\}"

TAG="${DIR##*/}"
TITLE="$TAG"

shopt -s nullglob dotglob

files=()
for path in "$DIR"/*; do
  [[ -f "$path" ]] && files+=("$path")
done

if [[ ${#files[@]} -eq 0 ]]; then
  echo "error: no files found in directory: $DIR" >&2
  exit 1
fi

if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  echo "release exists: $TAG"
else
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "$TITLE" \
    --notes "Automated release for $TAG"
fi

gh release upload "$TAG" "${files[@]}" \
  --repo "$REPO" \
  --clobber