# M10 — source NPC payment, hand-in and committed magic cues

Sources checked on 2026-11-08; Crystal revision
`201189f871d23597b82caf88fd611454a2ddefb8`:

- [A Sweaty Cyclops](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/a_sweaty_cyclops.lua), lines 283–338: four amulet pieces 7528–7531, 5,000 gold including bank, 24-hour assembly, then item 7532 once. No magic cue is invented for this step.
- [Oldrak](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/oldrak.lua), lines 117–145: progress 2, Axe 3274, 1,000 gold including bank, Hallowed Axe 919. `CONST_ME_YELLOWENERGY` is emitted at the **NPC**, not the player. The source permits repeated refreshes for a fresh axe/payment; no invented once-only flag.
- [Henricus](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/henricus.lua), lines 68–105 and 203–226: join, start the first mission, question the five guards, report. Outfit/addon and blessing purchase handlers are separate and are not duplicated here.
- [Kulag](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/kulag_the_guard.lua), lines 63–70: trouble → authorities; mark guard, increment Mission01, `CONST_ME_HOLYAREA` at player.
- [Grof](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/grof_the_guard.lua), lines 62–71: trouble → authorities; same committed counter/cue composition.
- [Tim](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tim_the_guard.lua), lines 61–84: trouble → authorities → avoided → gods would allow; same committed counter/cue composition.
- [Walter](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/walter_the_guard.lua), lines 61–81: same topic sequence, with his own literal replies and guard flag.
- [Miles](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/miles_the_guard.lua), lines 61–76: trouble → foresight of the authorities → also for the gods → trouble will arise in the near future.
- [Pinned Crystal items.xml](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml), SHA-256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`; the importer verifies this complete committed pin. The downloaded file remains outside Git; generated profiles retain the full actual pin.

Presentation identities reuse `content/presentations/bindings/client-ids.json`
(the existing SPELL-PRESENT-0 binding): yellow energy is 49, holy area is 50,
magic blue is 13. The executable resolver reads that qualified binding, rather
than assigning a new wire code. Publication uses the existing scoped magic-effect
journal after the durable transaction; refused/replayed value operations emit no
new cue. The immutable `game_quest_reward_sources.plan` records each cue together
with the payment, inputs and progress. Visual delivery is best effort; it never
turns a committed payment into a refusal.
