# M07 shared Fire Bug entrance

M07 remains PARTIAL for the carrot and Death Ring/tree-reset owners. This slice
implements Fire Bug ignition through real world-object USE and entry through the
existing dynamic-portal STEP owner. It creates no personal progress track and
completes no quest. Generic authored journey registration remains M01's boundary.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` references:

- `data-global/scripts/quests/others/actions_fire_bug.lua:8-27,99-109,172-173`:
  retained item5467 opens portal1949 at (32849,32233,z9), leading to
  (32857,32234,z11), with eight yellow-ring cues and a five-minute ashes reset.
- `data-global/scripts/quests/dreamers_challenge_quest/movement_fire_bug_teleport.lua:9-19`
  is a separate UID2243 movement handler. Ignition creates an unkeyed native
  teleport, so this UID handler is not used to close it after one traversal.
- `src/utils/utils_definitions.hpp:63,66` binds yellow rings8 and teleport11.

The literal ashes position is source-corroborated: `revertAshes` creates ashes3134
there and assigns UID2243. The retained bare B3 counterpart is ordinal1. USE
requires those current ashes and a real carried Fire Bug. Busy portals refuse
before durable admission. Missing inputs leave no receipt or projection.

The existing fenced Quest writer now admits this exact empty-progress action
only with world context and the registered retained-item callback. Migration2601
adds an immutable same-transaction proof tied to its Quest receipt. It verifies
current live custody, the unchanged item and location, source position, active
content/frame binding and reset epoch. Existing receipt shape predicates and
value-only guard branches are preserved. No consumption record or replacement
item is manufactured for the retained tool.

`TimedEntrance` publishes the glyph transform, native route and reset together
under the Channel facts owner. The route owns the same source revision as its
reset; traversal does not change that revision or deadline. Expiry retires the
route and restores ashes. An intervening source edit disables the stale route
and fences its old reset. The base glyph remains the sole rendered object.
Ignition cues publish only with a fresh receipt; replay extends neither cues nor
the deadline. The deadline starts after durable success.

The native generator adds only `oteryn:item.tibia.i5467`: base profiles5782→5783,
compiled cohort5798→5799, with no existing record changed or removed. The pinned
Crystal and retained Canary XML node SHA256 is
`6e788b59b3e1c3b5b68d0decc30a210007db94c63d4346f5352036562d94d3a4`.
Its exact source/appearance pair qualifies native custody without hand-editing
canonical unknown admission. Provider, cohort and dependency pins are regenerated.

Validation: the mounted PostgreSQL18.6 case passes through real USE and STEP,
including missing input, retained custody, empty progress, busy refusal, replay,
five-minute expiry, traversal without closing, presentation and hash-valid forged
source/content/frame proofs. World units have 250 passes and one pre-existing
ignored case; the new shared entrance lifecycle case passes. All three focused
Dreamer Quest tests pass. Four-crate checks and Clippy pass. Provider suites have
8 and13 passes, and native generation freshness passes. The unrelated Ice Islands
`published_ungated_quest_hash_baseline` mismatch remains unchanged. Local node boot
is skipped as instructed; no full-quest or production-map collision claim is made.
The exact native equipment/cohort case also passes. All four mounted M07 cases
return successfully without the PostgreSQL admin URL; the filtered route-less run
has five passes including the existing area-census unit case.
