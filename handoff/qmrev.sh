#!/bin/bash
# usage: qmrev.sh <lane>  -> prints review info, writes merged tree to /tmp/qmrev-<lane>.tree
L=$1; cd /tmp/mm42/r
git fetch -q origin +refs/heads/claude/happy-davinci-ijz3wy-qm-$L:refs/remotes/origin/x-qm-$L +refs/heads/claude/happy-davinci-ijz3wy-qmech:refs/remotes/origin/x-qmech || exit 1
B=origin/x-qmech; H=origin/x-qm-$L
echo "qmech=$(git rev-parse --short $B) head=$(git rev-parse --short $H)"
git log --format='%h %ae %s' $B..$H | head -8
git diff --stat $B...$H | tail -40
T=$(git merge-tree --write-tree $B $H) || { echo CONFLICT; echo "$T" | head; exit 2; }
echo $T > /tmp/qmrev-$L.tree; echo tree=$T
rm -rf /tmp/fmtq; mkdir -p /tmp/fmtq; git show $B:rustfmt.toml > /tmp/fmtq/rustfmt.toml
for f in $(git diff --name-only --diff-filter=AM $B $T -- '*.rs'); do mkdir -p /tmp/fmtq/$(dirname $f); git show $T:$f > /tmp/fmtq/$f; (cd /tmp/fmtq; out=$(rustfmt --check --edition 2024 --config-path rustfmt.toml $f 2>&1); [ $? -ne 0 ] && ! echo "$out" | grep -q 'failed to resolve mod' && echo "FMT $f"); done
echo "-- unwrap/expect/ignore added:"; git diff $B...$H -- '*.rs' | grep -E '^\+' | grep -cE '\.unwrap\(\)|\.expect\(|#\[ignore|todo!\(|unimplemented!\('
echo "-- migrations added:"; git diff --name-only --diff-filter=A $B...$H -- '*migrations/*.sql'
echo "-- duplicate mod/fn decls in changed .rs:"
for f in $(git diff --name-only --diff-filter=AM $B $T -- '*.rs'); do git show $T:$f | grep -E '^\s*(pub(\(\w+\))? )?(mod \w+[ ;{]|fn \w+)' | sed 's/[({].*//' | sort | uniq -d | sed "s|^|DUP $f: |"; done
