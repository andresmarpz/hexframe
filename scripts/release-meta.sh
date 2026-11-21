#!/usr/bin/env bash
# Prints the release title or notes for a version from its changelog entry.
set -euo pipefail
what="$1"
version="${2#v}"
entry="apps/web/src/content/changelog/$version.md"
[ -f "$entry" ] || { echo "missing changelog entry $entry" >&2; exit 1; }
title=$(sed -n 's/^title: //p' "$entry" | head -1)
case "$what" in
  title)
    case "$version" in
      *-preview.*) echo "Preview ${version##*-preview.} · $title" ;;
      *) echo "Greyscreen $version · $title" ;;
    esac ;;
  notes) awk 'seen >= 2 { print } /^---$/ { seen++ }' "$entry" | sed '/./,$!d' ;;
  *) echo "usage: release-meta.sh title|notes <version>" >&2; exit 1 ;;
esac
