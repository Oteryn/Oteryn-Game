# Spell visual/audio runtime boundary

The source cue table and committed owner outbox already preserve numeric effects,
missiles and sounds. This is data readiness, not a claim that the native client
renders or plays every spell. `audit.json` records current source hashes and
separate activation flags.

The accepted protocol registry does not allocate `PRESENTATION_V1`,
`WORLD_PRESENTATION` or `ACTOR_COOLDOWNS`. The strict codec and observer publisher
are explicitly local candidates. `CandidatePresentationTurn` and
`CandidateSessionPublisher` have test/qualification callers but no serving-node
egress caller. Session, dev-client and native client have no presentation
consumer; native client has no audio playback dependency. The existing VIS-2
publisher refuses negative native floors rather than inventing a Thalom frame
projection. Words/value shapes exist in the codec but do not yet have actual
committed outcome producers.

The pinned tracked 15.30 asset manifest has 6,249 entries, zero conventional audio
files and zero sound-named entries. That finding covers this manifest only; it
says nothing about other assets on the owner's computer. Numeric sound IDs alone
do not prove sound-file admission or playback.

Remaining integration order:

1. Accept owning protocol/resource registration and presentation capability gate.
2. Compose owner outbox egress under current session/content/frame authority.
3. Consume bounded presentation batches in Session and native client.
4. Admit source-qualified sprite/audio assets and render/play them.
5. Produce words and combat values from actual committed outcomes.
6. Qualify native-floor projection, reconnect/barrier/overflow and visual/audio E2E.

No new protocol IDs, replacement wire format, fabricated audio files or production
activation were introduced by this audit. Source data corrections can proceed
independently of these shared integration boundaries.

## Completed source qualification repair

The cue builder previously collected numeric names from both entire headers,
allowing an unrelated enum to supply an effect or sound ID. The regression
reproduced acceptance of `CONST_ME_FOREIGN=99` and `SPELL_FOREIGN=123` outside
those owning enums. It now reads only `MagicEffectClasses`, `ShootType_t` and
`SoundEffect_t`, rejects unsupported expressions/duplicate members/overflow,
and requires the actual pinned Lua registration. The Lua registration file is
included in source hash provenance. Four focused tests pass.

Regeneration from the final r25 player catalog preserves all 213 existing cue
records, aliases and numeric IDs exactly (51 effects, 14 projectiles, 148 sound
aliases). The original two header hashes are unchanged. This repair improves
source qualification; it does not activate the candidate protocol or client.

## Closed native-profile synchronization

The sound repair changes 13 of the 67 closed native headers. Their compiled
reference profiles now carry the exact new catalog presentation. All other
header fields, execution parameters and dependencies remain equal; the existing
`assemble` validator and strict Rust whole-header/dependency equality remain in
place. Current sample catalog and source-selection bytes equal the r25 candidate
copies. Historical r21-r24 evidence was not modified.

`complete_player_sounds.py` supports paired `--native-profiles` and
`--native-profiles-out` arguments. To reproduce the initial 67 catalog repairs
and 13 native-header repairs, supply the **pre-r25** catalog and native profiles
from the PR predecessor Git head, not the synchronized current samples. Running
against the synchronized samples is idempotent and reports zero additional
changes. The census is
`tools/content-schema/spell-authoring/samples/spell-census-canary-99902524-crystal-ff7ede5.json`.

Four synchronization tests cover exact current closure, presentation-only update,
mechanical/dependency substitution refusal and duplicate/unqualified input refusal.
Nine native-profile and eight executable-catalog tests also pass.
