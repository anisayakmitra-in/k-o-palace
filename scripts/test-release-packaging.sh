#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

archive_dir="$temp_dir/nested/dist"
archive="$archive_dir/k-o-palace-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
mkdir -p "$archive_dir"
printf 'release artifact\n' > "$archive"

"$root_dir/scripts/write-sha256-sidecar.sh" "$archive"

expected="$(sha256sum "$archive" | awk '{print $1}')  $(basename "$archive")"
actual="$(cat "$archive.sha256")"

if [[ "$actual" != "$expected" ]]; then
  printf 'expected checksum sidecar %q, got %q\n' "$expected" "$actual" >&2
  exit 1
fi

(cd "$archive_dir" && sha256sum -c "$(basename "$archive").sha256")
