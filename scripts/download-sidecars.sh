#!/usr/bin/env bash
# Downloads the sidecars of swarmy into assets/, named as Tauri expects them,
# `<name>-<target triple>[.exe]`:
#
# - rerun-cli, the Rerun Viewer of "Load Rerun Preview", from the rerun releases, the version must
#   match the `rerun` crate of Cargo.lock: https://github.com/rerun-io/rerun/releases
# - swarmy-bevy, the 3D map and replay views, from the swarmy-bevy releases:
#   https://github.com/sebosp/swarmy-bevy/releases
#
# Usage: scripts/download-sidecars.sh [target triple]
# The target triple defaults to the host one, SWARMY_BEVY_VERSION selects the swarmy-bevy release
# tag, the latest one by default.
set -euo pipefail

cd "$(dirname "$0")/.."
target="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
extension=""
if [[ "$target" == *windows* ]]; then
  extension=".exe"
fi

# The version of the rerun crate swarmy is built with, the viewer must be the same version.
rerun_version="$(awk '/^name = "rerun"$/ { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)"
if [[ -z "$rerun_version" ]]; then
  echo "The rerun crate is not in Cargo.lock" >&2
  exit 1
fi

download() {
  local url="$1" destination="$2"
  echo "Downloading $url"
  curl --fail --location --silent --show-error --output "$destination.part" "$url"
  mv "$destination.part" "$destination"
  chmod +x "$destination"
  echo "  -> $destination ($(wc -c < "$destination") bytes)"
}

download \
  "https://github.com/rerun-io/rerun/releases/download/${rerun_version}/rerun-cli-${rerun_version}-${target}${extension}" \
  "assets/rerun-cli-${target}${extension}"

swarmy_bevy_release="latest/download"
if [[ -n "${SWARMY_BEVY_VERSION:-}" ]]; then
  swarmy_bevy_release="download/${SWARMY_BEVY_VERSION}"
fi
download \
  "https://github.com/sebosp/swarmy-bevy/releases/${swarmy_bevy_release}/swarmy-bevy-${target}${extension}" \
  "assets/swarmy-bevy-${target}${extension}"
