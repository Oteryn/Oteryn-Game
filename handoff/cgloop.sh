#!/bin/bash
cd /tmp/mm42/r
git fetch -q origin '+refs/heads/claude/happy-davinci-ijz3wy-cg*:refs/remotes/origin/x-cg*' || exit 1
cur=$(git rev-parse origin/x-cgmech); start=$cur
export GIT_AUTHOR_NAME=blakinio GIT_AUTHOR_EMAIL=75369544+blakinio@users.noreply.github.com GIT_COMMITTER_NAME=blakinio GIT_COMMITTER_EMAIL=75369544+blakinio@users.noreply.github.com
for r in $(git for-each-ref --format='%(refname:short)' 'refs/remotes/origin/x-cg-*' 'refs/remotes/origin/x-cg_*'); do
  l=${r#origin/x-}; [ "$l" = cgmech ] && continue
  b=$(git rev-parse $r)
  git merge-base --is-ancestor $b $cur && continue
  mb=$(git merge-base $cur $b) || continue
  t=$(git merge-tree --write-tree --merge-base=$mb $cur $b 2>/dev/null) || { echo "CONFLICT $l $(git rev-parse --short $b)"; continue; }
  t=$(echo "$t" | head -1)
  [ "$t" = "$(git rev-parse $cur^{tree})" ] && continue
  s=$(git log --no-merges -1 --format=%s $mb..$b); [ -z "$s" ] && s="lane update"
  tag=$(echo ${l#cg} | tr -d '_-' | tr a-z A-Z)
  cur=$(printf 'CG-%s: %s\n\nSquash of claude/happy-davinci-ijz3wy-%s @ %s.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01D1SN2sEo2yY1k19f6kjEHf\n' "$tag" "$s" "$l" "$(git rev-parse --short $b)" | git commit-tree $t -p $cur)
  echo "SQUASH $l $(git rev-parse --short $b) -> $(git rev-parse --short $cur)"
done
[ $cur != $start ] && git push -q origin $cur:refs/heads/claude/happy-davinci-ijz3wy-cgmech && git ls-remote origin refs/heads/claude/happy-davinci-ijz3wy-cgmech
