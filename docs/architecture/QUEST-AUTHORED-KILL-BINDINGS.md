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
