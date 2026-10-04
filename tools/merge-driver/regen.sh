#!/bin/sh
# After `git merge origin/main`: regenerate every derived content file, then run the freshness checks.
# Leaves the result staged and uncommitted. Conflicts in hand-written files stop it for a person.
set -eu
cd "$(git rev-parse --show-toplevel)"
exec python3 tools/content-migration/regenerate_content.py --resolve "$@"
