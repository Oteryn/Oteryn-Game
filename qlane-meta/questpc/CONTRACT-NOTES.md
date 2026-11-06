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
