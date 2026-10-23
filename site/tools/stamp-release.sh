#!/usr/bin/env bash
# Keep the workflow entry point stable; all release/asset logic lives in JS.
set -euo pipefail
exec node "$(dirname "${BASH_SOURCE[0]}")/stamp-release.mjs"
