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


## Coordinator merge verification

Merged qmech `74bb50fcb`. Kept its M07 immutable-object admission guard and
added the legacy-empty Ground field to M07's new test initializer. No
assertion or existing admission predicate was weakened.

- Entire quests library: **352 passed**, **41.79s**; M02 regression rerun
  after selecting the source crate i2471: PASS.
- M02 PostgreSQL on the merged tree: PASS **5.126674382s**.
- All ten key33 checks: PASS **1.64s** after repository-tool refresh.
- `regenerate_content.py --native-only`: PASS; this engine slice changes no
  World content input, and regeneration left the canonical data unchanged.
- All-target clippy of quests, durability and dispatch passed before merging;
  all three production libraries passed again together after the merge in
  **2m28s** with `-D warnings`. `cargo fmt --all --check` passed.


## Source-specific stateless window slice

The selected Oteryn recipe's crates/amphorae variant now has a closed native
USE callback: `oteryn:quest-callback/m02/nomads-land/window`. This is a
chosen Oteryn physical placement, **not** a claim that the wiki provides its
literal original-world coordinates. The actual base region contains the
sandstone window1744 at `(33216,32855,9)`, base ordinal1; both north/northwest
cells `(33216,32854,9)` and `(33215,32854,9)` contain clear floor7358.
Native floors are `-9`. Region SHA256: `5f1c4081bc48d6e2481d0e29f37fed6b6823d6e749d1d9e75ecbfba15706d3bc`.
The ordinal is an existing base slot, not a newly allocated registry number.

Each selected stack is crate2471, crate2471, large amphora2904, quantity1
per root. The native Item provider already admits both definitions: crate is
NonStackable with CharacterInventory/Ground destinations; the large amphora
is NonStackable and Ground-only. No Item facts or inventory permissions were
invented. The source wiki raw body fetched on2026-11-08 has SHA256
`a5996f49a2db4e5965cb7867fe0e15916bec672becc15b0791f7db7b3eb81b1c`; lines115–117 describe the arrangement.
Other source variants and other Nomads Land stages are not certified here.

The existing current World observer now checks this closed read-set under
its real Character/node/map/content/reset fences and the retained spell
lane. USE, STEP and replay use the same fence decision. The window has no
personal storage, quest receipt, reward, teleport or synthetic progress flag;
its ordinary native USE feedback reports the physical check. A changed
reserved window callback is rejected by the content parser.

Real Root-observer RED: an empty stack was accepted; the assertion failed
after **5.28s**. GREEN initially **5.064853258s**. The extended positive/negative
PG pair then passed together: generic stack **9.351064254s**, actual World
window observer **9.397071216s**. Both piles are required, repeated checks are
read-only, an extra root or stale frame refuses, and Item quantities plus
quest tracks/receipts remain unchanged. This is mechanic-boundary evidence,
not whole-quest completion evidence.
