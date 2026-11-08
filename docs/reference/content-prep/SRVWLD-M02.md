# SRVWLD M02 — ordered mutable Ground prerequisites

Selected the first `OPEN`, `claim none` row of `qlane-meta/qmech/INDEX.md`
after M17. No migration or public number allocated; reservation remains
2190–2199. The coordinator owns the central claims file.

## Source and scope

The selected Nomads Land recipe's s4 is in
`content/quests/definitions/quests-00300-00351.json`, canonical identity
`oteryn:quest.authored.nomads_land_quest`. It derives from the pinned
TibiaWiki spoiler page 22842/revision1002600. The recipe explicitly asks for
crates and an amphora before approaching the cabinet.

Independent reference fetched 2026-11-08:
<https://www.tibiawiki.com.br/Arito%27s_Task_Quest?action=raw>, lines115–117
under “Nomads Land”: two stacks beside the window; either three crates per
stack, or two crates with an amphora on top. These are physical prerequisites,
not offerings which a quest consumes. The first engine slice does not invent
a window coordinate or claim the other Nomads Land stages are wired.

## First working slice

`WorldRequirement.ground_stack` specifies bounded root definitions and exact
quantities in bottom-to-top order. Legacy immutable-map requirements retain
their serialized form. Mutable stacks require a personal durable callback;
stateless/monster/reversible callbacks cannot bypass the SQL participant.

The existing Quest commit participant invokes the read-set before progress
or rewards. It validates exact scope/map/content/placement/generation custody,
locks Item identities before the existing Ground key33 owner, then rereads
custody in a new statement. A newly arrived unlocked root refuses admission.
No Item quantity, location, mint, burn or quest counter is changed by the read.

Red: the callback parser rejected the new bounded prerequisite. Green:
`m02_ground_stack_requires_exact_ordered_bounded_items_and_durable_admission`.
Real PostgreSQL:
`m02_ground_stack_postgres_exact_custody_order_quantity_and_no_consumption`
passed in **5.239794347s**, using the real native MINT owner and installed
migrations. It checks correct order/quantity, wrong order/quantity/position,
stale map/scope generation, an extra root, and unchanged Item quantities.
All ten repository-generated key33 inventory tests passed in **2.20s**.
The self-contained std-only test source was compiled directly with `rustc
--edition=2024 --test` and the normal game-server `CARGO_MANIFEST_DIR`; no
assertion or scan input was replaced.

Pending next slice: source-specific Nomads Land obstruction binding and a
complete quest-call regression. The generic read-set alone is not a whole
quest qualification.
