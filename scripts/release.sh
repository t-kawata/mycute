#!/usr/bin/env bash
set -euo pipefail

# APP_REPO 環境変数が設定されている場合はそれを使用し、なければデフォルトの mycute リポジトリを使用する。
# Makefile が apply-edition.js 生成の .env を source することで、エディションに応じたリポジトリに切り替わる。
REPO="${APP_REPO:-mycute-os/mycute}"
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
# APP_DISPLAY_NAME が設定されている場合は、その文字列を名前に含むファイルのみを対象とする。
# これにより、他方のエディションのファイルが誤ってアップロードされるのを防ぐ。
FILTER="${APP_DISPLAY_NAME:-}"
for path in "$DIR"/*; do
  if [[ -f "$path" ]]; then
    if [[ -z "$FILTER" ]] || [[ "$path" == *"$FILTER"* ]]; then
      files+=("$path")
    fi
  fi
done

if [[ ${#files[@]} -eq 0 ]]; then
  echo "error: no files found in directory: $DIR" >&2
  exit 1
fi

RELEASE_NOTES_FILE="release-notes/${TAG}.md"
if [ -f "$RELEASE_NOTES_FILE" ]; then
  NOTES_OPTION="--notes-file"
  NOTES_VALUE="$RELEASE_NOTES_FILE"
  echo "Using release notes from $RELEASE_NOTES_FILE"
else
  NOTES_OPTION="--notes"
  NOTES_VALUE="Automated release for $TAG"
fi

if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  echo "release exists: $TAG"
else
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "$TITLE" \
    "$NOTES_OPTION" "$NOTES_VALUE" \
    --latest
fi

gh release upload "$TAG" "${files[@]}" \
  --repo "$REPO" \
  --clobber

# Demote v2.x.x releases from "Latest" so that v0.x.x releases appear at the top.
gh release list --repo "$REPO" --json tagName,isLatest --jq '.[] | select(.isLatest and (.tagName | startswith("v2."))) | .tagName' |
while IFS= read -r tag; do
  echo "Demoting $tag from Latest..."
  gh release edit "$tag" --repo "$REPO" --latest=false
done