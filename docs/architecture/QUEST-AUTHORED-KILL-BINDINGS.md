# Authored kill bindings

A source-owned binding document can declare a stage with
`authored_bindings::Trigger::Kill { counter: Box<KillCounter> }`. Parse it with
`Bindings::parse` and attach it to the admitted `QuestStateCatalogue` using
`Bindings::attach`. This creates no creature or quest content implicitly.

`KillCounter` declares the transition `key`, exact native `creature`, count
`track`, separate `gate`, `gate_value`, inclusive count `minimum` (default 0),
`maximum`, and optional `advance_gate_to`. Both tracks must belong to the bound
quest and cover all declared values. The transition key must use
`oteryn:quest-transition/native-kills/`; retained counter keys cannot be replaced.
At most 128 kill bindings are admitted in one document.

The existing native death queue captures active counters, exact creature keys,
quest definition hashes, party membership and character revisions. Settlement
uses the existing session fence, root/track locks and occurrence receipts.
Inactive gates, different species, saturated counters and duplicate lethal
projections earn no credit. A queued death cannot enter a mission whose gate
was written after the captured character revision.

Every eligible death increments the count by one. `advance_gate_to` applies
only when the original locked count is at least `maximum - 1`; until then the
gate is preserved. A stage with `completes: true` marks completion only at the
count goal. Quest writers use `QuestStateCatalogue::completes_after` for this
conditional completion. Existing static counters retain their receipt binding
bytes.

Catalogue consumers can use `native_kills::counter_for`, `active_for` and
`tracks_creature_for`. The underlying `QuestEffectKind::SetWhenAtLeast` reads
another effect's original locked value and rejects self/out-of-range indices.
It reuses questpc3 commit `262618b3cd9fce9de09ee83064de6f31e95aa8e7`.

The shared synthetic fixture drives native combat and ordinary NPC dialogue in
`authored_kill_bindings::authored_bindings_native_kill_threshold_completion_postgres`.
It does not qualify the fixture's named packet quest.

The queued regression is `authored_bindings_queued_kill_burst_postgres`: three
real deaths are captured before settlement, then credited exactly once each.
Main commit `6738c1da7ad5cf35110145fe33dfa9ea377ab0d7` already preserves the mission epoch by removing unchanged
native effects before receipt/track writes (`quest_state/writer.rs:709–710`).
Ordinary mission and reset writes continue to advance the gate revision.

After merging main `6f61ac15d`, the authored threshold operation uses definition
encoding tag `11`, preserving main's published bound-set tag `7`. The merged
sequential/burst and retained Dragon Slayer PostgreSQL cases passed on `aa85b2500`
with completion markers of 122.941s, 118.673s, and 123.873s. No production quest binding
was added by these shared-engine tests.
