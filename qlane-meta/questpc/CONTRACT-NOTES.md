# Questpc contract decisions

- StepOut observes the committed movement's original tile and settles before
  destination StepIn. It is currently restricted to stateless map restoration;
  storage, offerings, experience, item-on-target and self-teleport are rejected
  by both Python and Rust. Existing occupied-wall refusal remains in force.
- A cancelled static departure with an exact matching pending record and a
  strictly newer source-object revision is superseded. Retire that event, keep
  the newer map state, and resume destination entry using the original movement
  snapshots. Never discard a durable callback or exhausted revision. PG RED/GREEN
  covers session recovery, destination feedback and unchanged tracks/receipts.
- This slice covers ordinary committed paced player movement. Source quest
  compositions and other relocation producers require their own qualification;
  no whole title was promoted by this engine test.
- Reused StepOut implementation from questpc5@2d9db9c5abdb62abc8f9883984b5ad5bc22244c8
  after checking qengine2@5a6de28c142de185594217ce1e5cbc7f4b6e6e4e.
- Next source facts: Devil Helmet creates/removes435 at32482,32170,z14;
  Mintwallin removes its lever, creates two1270 walls and removes nine1271 walls.
  Demon Helmet requires two player plates and ejects the top creature before
  recreating missing walls. Position:hasCreature calls getTopCreature and then
  teleportTo(destination,true); do not assume it moves every creature.

Sources:
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/devil_helmet/movements_sewer.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/demon_helmet/movement-walls.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data/libs/functions/position.lua#L192

Limit each future investigation to ten minutes; record the conservative decision
here and continue. Preserve the atomic eleven-track Father's Burden write and
exact-match charge grants.

## Atomic source objects

- At most16 trigger/object effects; immutable pinned palette facts only. Unknown,
  ambiguous, pickupable, durable Ground/House objects refuse the entire batch.
- Stage a clone of the existing callback state, then publish removals, creations,
  trigger revision and collision together. No durable item or Quest writer.
- Create only when that appearance is absent; missing removals are no-ops. This
  conservative rule avoids duplicate static walls and matches conditional sewer
  creation. Mintwallin's consumed trigger still makes its normal branch one-shot.
- Floor-changing creation currently refuses. Add composed-floor resolution before
  enabling435; rendering a hole without descent is not an accepted fallback.
- Mintwallin's201189 source is byte-identical to retained9f5a72c6 (SHA25661944b37...).
  Bind its exact AID30008; no arbitrary source-coordinate alias.
  https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua

- The current generated floor catalogue does not contain435. Explicitly refuse
  that appearance until its source floor facts and composed routing are implemented;
  absence from the catalogue must not silently make a sewer visual-only.

## Mintwallin qualification

- Production selector remains only AID30008, exactly as the source. The controlled
  test lever at32592,32107,z14 is a fixture anchor, not a production position alias.
- All11 declared mutation targets plus consumed trigger execute atomically. The
  scene builder includes empty targettiles; a missing tile continues to refuse.
- Source packet count grows21->22 with Mintwallin. Exact named-input gate and
  corresponding expected-count tests grow together; no gate is weakened.
- Fresh questpc8@15b02adfd status for Mintwallin isUNFINISHED. This lane's source
  qualification does not certify a deployed complete World route or unrelated
  authored stage approximations.
