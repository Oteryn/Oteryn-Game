# Requests sent under the latest owner boundary

ENGINE-REQUEST questpc8: carried USE item replacement/reward and a fenced live-player quest-state predicate | needed by: The Colours of Magic | where: crates/gameplay/quests, crates/gameplay/dispatch, crates/gameplay/durability

The chosen recipe needs an actual blue-marked actor paired with another current,
yellow-marked player, then its once-only reward. Preserve selected-instance USE
proof and reject self, stale sessions and cross-scope targets. Source recipe:
https://tibia.fandom.com/index.php?curid=50860&oldid=1128664

ENGINE-REQUEST questpc8: source item-look disguises (lookTypeEx) with appearance restoration | needed by: Wrath of the Emperor Quest | where: crates/gameplay/quests, crates/gameplay/dispatch

Crystal's crate action uses lookTypeEx11328 with indefinite outfit condition;
the current callback disguise accepts a creature identity, not an item look.
Do not replace this with a fabricated creature or bypass the disguise stage.
Source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_crate.lua

QENGINE2 owns shared engine changes under the owner's new rule. Both requests
were sent in the conversation; no large generic replacement is implemented here.
The packet still lists these two, but the ownership coordinator may transfer
not-started entries to the new lanes. Reuse any existing work and await that
explicit ownership/merged-engine notice before duplicating their implementation.

Rechecked main47db34f09 and QENGINE2 279a9cff8 on2026-11-07. QENGINE2's
new canonical NPC reward/input repairs do not implement either selected-peer
colour predicates or lookTypeEx item disguises. Reuse the canonical main NPC
implementation already merged here; retain the two requests above rather than
creating another shared engine owner. Wrath's exact-material opening slice is
implemented and has its own native PostgreSQL proof; that does not qualify the
later disguise graph or the entire quest.
