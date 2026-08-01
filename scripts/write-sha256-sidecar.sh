#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  echo "usage: write-sha256-sidecar.sh <archive>" >&2
  exit 2
fi

archive="$1"
if [[ ! -f "$archive" ]]; then
  echo "archive not found: $archive" >&2
  exit 1
fi

archive_dir="$(cd "$(dirname "$archive")" && pwd)"
archive_name="$(basename "$archive")"

(
  cd "$archive_dir"
  sha256sum --text "$archive_name" > "$archive_name.sha256"
)
