#!/bin/bash
# usage: qmres.sh <lane> path=resolvedfile ...   -> writes /tmp/qmrev-<lane>.tree from conflicted merge-tree with resolutions
L=$1; shift; cd /tmp/mm42/r
B=origin/x-qmech; H=origin/x-qm-$L
T=$(git merge-tree --write-tree $B $H | head -1)
export GIT_INDEX_FILE=/tmp/qmres-idx; rm -f $GIT_INDEX_FILE; git read-tree $T
for pr in "$@"; do p=${pr%%=*}; r=${pr#*=}; grep -nE '^(<<<<<<<|>>>>>>>|=======$)' $r && { echo "MARKERS in $r"; exit 2; }; h=$(git hash-object -w $r); git update-index --cacheinfo 100644,$h,$p; done
T2=$(git write-tree); unset GIT_INDEX_FILE
left=$(git ls-tree -r $T2 --name-only | while read f; do :; done; git diff --name-only $B $T2 | while read f; do git show $T2:$f 2>/dev/null | grep -lqE '^(<<<<<<<|>>>>>>>) ' && echo $f; done)
[ -n "$left" ] && { echo "UNRESOLVED: $left"; exit 3; }
echo $T2 > /tmp/qmrev-$L.tree; echo tree=$T2
