#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

assert_fails() {
  if "$@"; then
    printf 'expected command to fail:' >&2
    printf ' %q' "$@" >&2
    printf '\n' >&2
    exit 1
  fi
}

release_repo="$temp_dir/release-repo"
git init --bare "$temp_dir/origin.git" >/dev/null
git init -b main "$release_repo" >/dev/null
git -C "$release_repo" config user.name "Release Gate Test"
git -C "$release_repo" config user.email "release-gate@example.invalid"
cat > "$release_repo/Cargo.toml" <<'EOF'
[package]
name = "release-gate-test"
version = "1.2.3"
EOF
git -C "$release_repo" add Cargo.toml
git -C "$release_repo" commit -m "release source" >/dev/null
git -C "$release_repo" tag v1.2.3
git -C "$release_repo" remote add origin "$temp_dir/origin.git"
git -C "$release_repo" push origin main --tags >/dev/null

(
  cd "$release_repo"
  bash "$root_dir/scripts/verify-release-source.sh" v1.2.3
  assert_fails bash "$root_dir/scripts/verify-release-source.sh" v1.2.4

  git switch -c unmerged >/dev/null
  sed -i 's/version = "1.2.3"/version = "1.2.3-unmerged"/' Cargo.toml
  git add Cargo.toml
  git commit -m "unmerged tag" >/dev/null
  git tag v1.2.3-unmerged
  assert_fails bash "$root_dir/scripts/verify-release-source.sh" v1.2.3-unmerged
)

dist_dir="$temp_dir/dist"
mkdir -p "$dist_dir"
printf 'linux archive\n' > "$dist_dir/k-o-palace-v1.2.3-linux.tar.gz"
printf 'windows archive\n' > "$dist_dir/k-o-palace-v1.2.3-windows.zip"
"$root_dir/scripts/write-sha256-sidecar.sh" "$dist_dir/k-o-palace-v1.2.3-linux.tar.gz"
"$root_dir/scripts/write-sha256-sidecar.sh" "$dist_dir/k-o-palace-v1.2.3-windows.zip"
bash "$root_dir/scripts/verify-release-checksums.sh" "$dist_dir"

printf 'tampered\n' >> "$dist_dir/k-o-palace-v1.2.3-linux.tar.gz"
assert_fails bash "$root_dir/scripts/verify-release-checksums.sh" "$dist_dir"
"$root_dir/scripts/write-sha256-sidecar.sh" "$dist_dir/k-o-palace-v1.2.3-linux.tar.gz"
rm "$dist_dir/k-o-palace-v1.2.3-windows.zip.sha256"
assert_fails bash "$root_dir/scripts/verify-release-checksums.sh" "$dist_dir"
