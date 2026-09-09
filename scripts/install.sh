#!/usr/bin/env sh
# Install the prebuilt BanditRS wheel for this platform, no Rust toolchain
# needed. Resolves the latest GitHub Release automatically, so this script
# never needs editing when a new version ships:
#
#   curl -LsSf https://raw.githubusercontent.com/LePhilippeDucTai/BanditRS/main/scripts/install.sh | sh
#
# `pip` is deliberate, not `uv`: as of uv 0.8, `uv`'s downloader 401s when a
# GitHub Release asset redirects to a signed objects.githubusercontent.com
# URL, while `pip` fetches the identical URL without issue. Once that's fixed
# upstream this can switch. `uv`/`uvx` still work fine for the git+ (build
# from source) install path documented in the README.
set -eu

repo="LePhilippeDucTai/BanditRS"

release_info=$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest")
tag=$(printf '%s\n' "$release_info" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "${tag:-}" ]; then
    echo "error: could not resolve the latest ${repo} release from the GitHub API" >&2
    exit 1
fi

pip="${PIP:-pip}"

# The "expanded_assets" page is an unlisted but long-stable GitHub endpoint
# that serves a plain list of <a href> download links for one release — the
# same shape a real package index would return. `pip` reads it exactly like
# one and picks the wheel matching this platform's OS, architecture and
# Python's own wheel tags, with no per-platform logic needed here.
"$pip" install --no-index \
    --find-links "https://github.com/${repo}/releases/expanded_assets/${tag}" \
    banditrs

echo "Installed banditrs ${tag#v}. Try: bandit --version"
