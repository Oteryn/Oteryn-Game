# Explicit native aliases for source reward placements

`content/quests/native-reward-placement-aliases.json` supplements an existing
source selector when the committed native map does not retain its startup UID.
It contains no reward or quantity fields. The canonical claim, revision,
placement key, item plan, charge/fluid semantics and character-once claim ledger
remain unchanged.

Each declaration identifies the existing claim/revision, expected Crystal source
UID, project position, appearance and native ordinal, with source citations.
Declarations are limited to64 and native ordinals to0..63. Duplicate source
selectors or duplicate native targets refuse the complete declaration set.
The binder validates every declaration against exactly one original source
placement; an unknown claim or a mismatched revision, UID, position or appearance
refuses the pass. Item admission is still checked before binding.

The existing strict UID selector runs first. An explicit alias is considered
only for `NoEntry`, only for its declared source placement, and only when the
entire target tile has no unique tags. Conflicting tags, ambiguous source tags,
wrong appearances and wrong ordinals do not become fallback grants. The public
arbitrary-shard binder remains strict; only the embedded approved registry uses
aliases. Existing legacy selectors retain priority and normal behavior.

The first alias is the Mintwallin Cyclops Ruby cache: original claim
`oteryn:reward-claim.kv.mintwallincyclops`, revision`reward-claim-r1`, source UID6138,
position32591,32097,14, appearance2472, ordinal1. The native tile contains ground351
and that chest, with no retained unique attributes. The unchanged source reward
is one item3028. Its original once-only identity is preserved across both paths.

Source: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua
