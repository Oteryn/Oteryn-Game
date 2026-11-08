# QENGINE2 qualified-snapshot transfer

Source: `aed1816b5` (the last qualified shared-engine snapshot). The later
`790996f8e` world-clock WIP and its follow-up are deliberately not imported.
No whole quest is certified or newly labelled DONE by this transfer.

Reusable APIs carried by the snapshot:

- `quest::authored_bindings::Bindings::{parse, parse_documents, attach}` and
  `verify_embedded_recipe_evidence`: catalogue-scoped authored NPC/carried USE,
  map USE/STEP and native kill-counter bindings with canonical recipe evidence.
- `quest::authored_bindings::KillCounter`: bounded, replay-safe death progress;
  participant-room predicates are not part of this snapshot.
- `reward_callbacks::RewardCallback::carried_exchange`: exact carried item
  exchanges composed with quest effects in the existing transaction.
- `world_callbacks::CallbackEffects::item_reward`: admitted World item delivery
  through the existing MINT owner, with rollback/replay protection.
- `world_reward_claims::{bind_reward_claims, bind_embedded_reward_claims}`:
  definition-ready claim/use resolution and exact placement admission.
- `RewardClaimMintRequest::additional_items: Vec<RewardClaimMintContent>`:
  multiple independent item roots in a single MINT receipt. Quest callback
  authoring for multiple outputs is separate from this primitive; qready2's
  claim remains its own work and should reuse this participant.

The duplicate source 1550 migration is omitted: qready2 migration 2100 and
its existing sibling MINT implementation are reused. Source migrations 1551
and 1552 are numbered 2041 and 2042 within the assigned 2040–2049 range.
Their SQL bodies are unchanged; historical proofs retain source
commit numbering. No new schema is planned for M01a.
Existing package migrations 1600 and 1641 and their owners are retained.
Package integration preserves sixteen-track NPC/reward transactions, literal
reward text, the injected quest clock, and current seasonal behavior.

Historical real-PG and unit evidence is retained in the imported
`qlane-meta/qengine2/PG-*.json` records, anchored to their original source
commits. It must not be relabelled as a fresh QMECH run. Current-package
qualification is still pending while this merge is prepared; no clock WIP or
whole-quest qualification is implied by the archived evidence.
