# Fanfare collection entry

The accepted donor claim in
`tools/content-schema/quest-authoring/samples/chests/claims.json:8197`
(commit `9e5c8994d2784bb29d1013d7379f4f26c3f2e98b`) binds
`canary:door-key/3520` to one `canary:item/2973`.
The generator verifies that exact pair before emitting the native collection
predicate. Item 2973 without key number 3520 does not satisfy it.

This implements only the chosen recipe's initial collection step. It checks
current carried custody after a committed native item move or reward-claim MINT and consumes
nothing. It does not prove which historical box supplied the key. The crypt
route, door interaction, final reward and completion remain unqualified.
