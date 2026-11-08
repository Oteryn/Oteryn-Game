#!/bin/bash
# usage: qmcommit.sh <lane> "<subject>" [files-to-rustfmt...]
L=$1; S=$2; shift 2; cd /tmp/mm42/r
T=$(cat /tmp/qmrev-$L.tree); H=$(git rev-parse --short origin/x-qm-$L)
export GIT_INDEX_FILE=/tmp/qm-idx; rm -f $GIT_INDEX_FILE; git read-tree $T
for f in "$@"; do git show $T:$f | rustfmt --edition 2024 --config-path /tmp/fmtq/rustfmt.toml --emit stdout > /tmp/fmtout.rs || exit 3; h=$(git hash-object -w /tmp/fmtout.rs); git update-index --cacheinfo 100644,$h,$f; done
T2=$(git write-tree); unset GIT_INDEX_FILE
export GIT_AUTHOR_NAME=blakinio GIT_AUTHOR_EMAIL=75369544+blakinio@users.noreply.github.com GIT_COMMITTER_NAME=blakinio GIT_COMMITTER_EMAIL=75369544+blakinio@users.noreply.github.com
P=$(git rev-parse origin/x-qmech)
C=$(printf '%s\n\nSquash of claude/happy-davinci-ijz3wy-qm-%s @ %s.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01D1SN2sEo2yY1k19f6kjEHf\n' "$S" "$L" "$H" | git commit-tree $T2 -p $P)
git push -q origin $C:refs/heads/claude/happy-davinci-ijz3wy-qmech || exit 4
git ls-remote origin refs/heads/claude/happy-davinci-ijz3wy-qmech
echo "$(date -u +%H:%M) $L $H -> $C" >> /tmp/clscr/qm3/merged.log
