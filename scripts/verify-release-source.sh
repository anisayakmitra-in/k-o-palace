#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  echo "usage: verify-release-source.sh <tag>" >&2
  exit 2
fi

release_tag="$1"
package_version="$({
  in_package=0
  while IFS= read -r line; do
    if [[ "$line" == "[package]" ]]; then
      in_package=1
      continue
    fi
    if [[ "$line" == \[*\] ]]; then
      in_package=0
    fi
    if (( in_package )) && [[ "$line" =~ ^version[[:space:]]*=[[:space:]]*\"([^\"]+)\"[[:space:]]*$ ]]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
      break
    fi
  done < Cargo.toml
} )"

if [[ -z "$package_version" ]]; then
  echo "Cargo.toml [package] version not found" >&2
  exit 1
fi

expected_tag="v$package_version"
if [[ "$release_tag" != "$expected_tag" ]]; then
  echo "release tag must be exactly $expected_tag, got $release_tag" >&2
  exit 1
fi

tag_commit="$(git rev-parse --verify "refs/tags/$release_tag^{commit}")"
head_commit="$(git rev-parse --verify HEAD^{commit})"
if [[ "$tag_commit" != "$head_commit" ]]; then
  echo "checked out HEAD is not $release_tag" >&2
  exit 1
fi

git rev-parse --verify refs/remotes/origin/main^{commit} >/dev/null
if ! git merge-base --is-ancestor "$tag_commit" refs/remotes/origin/main; then
  echo "$release_tag is not reachable from origin/main" >&2
  exit 1
fi
