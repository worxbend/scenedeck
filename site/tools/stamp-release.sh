#!/usr/bin/env bash
# Rewrite the version numbers baked into the static site so that download links
# always point at the newest published release.
#
# The site is authored with real, working URLs for whatever release was current
# when it was written (e.g. scenedeck-0.1.21-linux-amd64.flatpak), so it can be
# opened straight from disk during development. At deploy time this script
# rewrites those literals in place. If the release lookup fails for any reason
# the site is left exactly as committed, which still works — it just points at
# an older release.
#
# Usage: site/tools/stamp-release.sh   (run from the repository root)

set -euo pipefail

repo="${GITHUB_REPOSITORY:-worxbend/scenedeck}"
site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_tag() {
    if command -v gh >/dev/null 2>&1; then
        gh api "repos/${repo}/releases/latest" --jq .tag_name 2>/dev/null && return 0
    fi
    curl -sSfL "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null |
        sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
        head -n 1
}

tag="$(resolve_tag || true)"

if [[ -z "${tag}" ]]; then
    echo "stamp-release: could not resolve the latest release tag; leaving the site unchanged." >&2
    exit 0
fi

version="${tag#v}"

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "stamp-release: '${tag}' is not a vX.Y.Z tag; leaving the site unchanged." >&2
    exit 0
fi

echo "stamp-release: stamping ${site_dir} with ${tag}"

# Release asset file names, e.g. scenedeck-0.1.21-linux-amd64.AppImage
find "${site_dir}" -type f \( -name '*.html' -o -name '*.js' -o -name '*.json' \) -print0 |
    xargs -0 sed -i -E "s/scenedeck-[0-9]+\.[0-9]+\.[0-9]+-linux/scenedeck-${version}-linux/g"

# Human-readable version badges: <... data-release-version ...>v0.1.21</...>
find "${site_dir}" -type f -name '*.html' -print0 |
    xargs -0 sed -i -E "s/(data-release-version[^>]*>)v?[0-9]+\.[0-9]+\.[0-9]+/\1v${version}/g"

# The JSON snapshot the page reads for its live download panel.
if [[ -f "${site_dir}/release.json" ]]; then
    printf '{\n  "tag": "%s",\n  "version": "%s"\n}\n' "${tag}" "${version}" >"${site_dir}/release.json"
fi

echo "stamp-release: done"
