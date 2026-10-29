#!/usr/bin/env bash
# Print migration numbers (first four digits) that appear more than once on a ref.
set -euo pipefail
ref="${1:-origin/main}"
git ls-tree --name-only "$ref" apps/game-server/migrations/ \
  | xargs -r -n1 basename | cut -c1-4 | sort | uniq -d
