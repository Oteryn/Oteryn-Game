#!/bin/sh
# Verify the six MAINMERGE6 branch tips without changing the checkout.
set -eu

fix=3ffe8d05116b56d752944370a7c43341e0ca8bc8
prefix=refs/heads/claude/happy-davinci-ijz3wy-
set -- "${prefix}npcplace" "${prefix}overlordcr" "${prefix}vocreate" \
    "${prefix}cwarlock" "${prefix}numkeys" "${prefix}mp09s"

git fetch --no-tags origin refs/heads/main "$@"
remote_refs=$(git ls-remote --exit-code --refs origin refs/heads/main "$@")
main=$(printf '%s\n' "$remote_refs" | awk '$2 == "refs/heads/main" { print $1 }')
if ! git merge-base --is-ancestor "$fix" "${main:?origin/main is missing}"; then
    printf 'Cannot verify that origin/main (%s) contains #3303 (%s).\n' "$main" "$fix" >&2
    exit 1
fi
printf 'main\t%s\n' "$main"

failed=0
for ref do
    head=$(printf '%s\n' "$remote_refs" | awk -v ref="$ref" '$2 == ref { print $1 }')
    if [ -z "$head" ]; then
        printf 'Missing remote ref %s\n' "$ref" >&2
    elif ! git merge-base --is-ancestor "$fix" "$head"; then
        printf 'Cannot verify that %s (%s) contains #3303 (%s).\n' "$ref" "$head" "$fix" >&2
    elif merge_result=$(git merge-tree --write-tree "$main" "$head"); then
        printf '%s\t%s\t%s\n' "$head" "$ref" "$merge_result"
        continue
    else
        printf 'merge-tree exited %s for %s (%s):\n%s\n' "$?" "$ref" "$head" "$merge_result" >&2
    fi
    failed=1
done
exit "$failed"
