#!/usr/bin/env bash
# Show the last post-merge "Game Merge Queue gate" runs on main and which leaf
# jobs failed, so a merge decision starts from the real state of main.
# Usage: main-health.sh [runs]   (default 10; needs gh with REST access)
set -euo pipefail
n="${1:-10}"
repo="${OTERYN_REPO:-Oteryn/Oteryn-Game}"
wf=merge-group-gate.yml

gh api "repos/$repo/actions/workflows/$wf/runs?branch=main&event=push&per_page=$n" \
  --jq '.workflow_runs[] | [.id, .head_sha[0:10], .created_at[0:16], (.conclusion // .status)] | @tsv' |
while IFS=$'\t' read -r id sha at state; do
  failed=""
  if [ "$state" = "failure" ]; then
    failed=$(gh api "repos/$repo/actions/runs/$id/jobs?per_page=100" \
      --jq '[.jobs[] | select(.conclusion == "failure" and .name != "game-gate") | .name | sub("^Merge Queue / "; "")] | join("; ")')
  fi
  printf '%s  %s  %-9s %s\n' "$at" "$sha" "$state" "$failed"
done
