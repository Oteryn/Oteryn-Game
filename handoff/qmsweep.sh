#!/bin/bash
cd /tmp/mm42/r
git fetch -q origin '+refs/heads/claude/happy-davinci-ijz3wy-qm-*:refs/remotes/origin/x-qm-*' +refs/heads/claude/happy-davinci-ijz3wy-qmech:refs/remotes/origin/x-qmech
for r in $(git for-each-ref --format='%(refname:short)' 'refs/remotes/origin/x-qm-*'); do
  l=${r#origin/x-qm-}
  git merge-base --is-ancestor $r origin/x-qmech && continue
  out=$(bash /tmp/qmrev.sh $l 2>&1)
  if echo "$out" | grep -q '^CONFLICT'; then echo "CONFLICT $l $(git rev-parse --short $r): $(git merge-tree --write-tree origin/x-qmech $r 2>&1 | sed -n 's/^CONFLICT ([a-z/ ]*): Merge conflict in //p' | tr '\n' ' ')"; continue; fi
  t=$(cat /tmp/qmrev-$l.tree)
  [ "$t" = "$(git rev-parse origin/x-qmech^{tree})" ] && continue
  echo "$out" | grep -q 'migrations added:' && mig=$(echo "$out" | sed -n '/migrations added:/,/duplicate/p' | grep '\.sql') || mig=""
  s=$(git log --no-merges -1 --format=%s origin/x-qmech..$r); [ -z "$s" ] && s="lane update"
  fm=$(echo "$out" | sed -n 's/^FMT //p')
  if bash /tmp/qmcommit.sh $l "$s" $fm >/dev/null 2>&1; then echo "MERGED $l $(git rev-parse --short $r) $s ${mig:+MIG:$mig}"; else echo "FAILCOMMIT $l"; fi
done
git ls-remote origin refs/heads/claude/happy-davinci-ijz3wy-qmech
