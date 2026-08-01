#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  echo "usage: verify-release-checksums.sh <dist-directory>" >&2
  exit 2
fi

dist_dir="$(cd "$1" && pwd)"
shopt -s nullglob
archives=("$dist_dir"/*.tar.gz "$dist_dir"/*.zip)
sidecars=("$dist_dir"/*.sha256)

if [[ "${#archives[@]}" -eq 0 || "${#sidecars[@]}" -eq 0 ]]; then
  echo "release archives and checksum sidecars are required" >&2
  exit 1
fi

for archive in "${archives[@]}"; do
  if [[ ! -f "$archive.sha256" ]]; then
    echo "checksum sidecar missing for $(basename "$archive")" >&2
    exit 1
  fi
done

for sidecar in "${sidecars[@]}"; do
  archive_name="$(basename "${sidecar%.sha256}")"
  sidecar_name="$(basename "$sidecar")"
  if [[ ! -f "$dist_dir/$archive_name" ]]; then
    echo "$sidecar_name does not describe a release archive" >&2
    exit 1
  fi

  mapfile -t lines < <(tr -d '\r' < "$sidecar")
  if [[ "${#lines[@]}" -ne 1 ]]; then
    echo "$sidecar_name must contain exactly one checksum" >&2
    exit 1
  fi

  checksum="${lines[0]:0:64}"
  descriptor="${lines[0]:64}"
  if [[ ! "$checksum" =~ ^[0-9a-fA-F]{64}$ ]] ||
    { [[ "$descriptor" != "  $archive_name" ]] && [[ "$descriptor" != " *$archive_name" ]]; }; then
    echo "$sidecar_name must contain one SHA-256 for $archive_name" >&2
    exit 1
  fi

  (
    cd "$dist_dir"
    sha256sum --check --strict -- "$sidecar_name"
  )
done
