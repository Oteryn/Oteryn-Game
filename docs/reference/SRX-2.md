# SRX-2: Newhaven starter equipment and charm consumers

Task base: `faf8ae8700d935bca612261730aca5466526389a`.
This task does not change casting (S17), quest content, or the mechanics status ledger.

## Scope disposition

- C19b: Newhaven tutorial profession kit and first-entry runtime grant.
- P01: runtime consumers for Carnage, Cripple, Numb, Adrenaline Burst, Cleanse,
  Fatal Hold, Void Inversion, Bless, and Scavenge.
- S16c: **EXCLUDED** for Dawnport-only trial equipment, outfit and spells;
  non-Dawnport trial remainder **N/A**. No Dawnport grants are transplanted.
- M11a/M11b: dropped; already implemented by SV8, PR #3002.
- W08: dropped; owned by SV8. P11/P12a: dropped; parallel task owns them.

## S16c source audit

Retained Crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/movements/others/dawnport_vocation_trial.lua:245-317`
grants trial equipment, outfit and spells. Its step event is explicitly Dawnport
at lines 320-323, refuses level 20 and above at line 334, and registers only its
Dawnport action IDs at lines 358-362. The source tile block is explicitly
Dawnport (`data-global/startup/tables/tile.lua:780-793`).

Canary revision `04b83b512114bfd888000d6e1433ed8ecaec7c5b`,
`data-otservbr-global/scripts/movements/others/dawnport_vocation_trial.lua:284-311`
independently identifies the same step event, level gate, items and outfit.
These retained donor revisions establish the scope of the excluded mechanic;
they are not represented as newer re-audit pins or independent current Tibia proof.

The Newhaven picker is permanent first-profession selection: Crystal
`src/server/network/protocol/protocolgame.cpp:1250-1252,2029-2032` routes the
packet to `playerChangeVocation`; `src/game/game.cpp:1432-1469` refuses an
already-professioned character and publishes the first selection, without trial
grants. `src/creatures/players/player.cpp:125-135` updates profession and
regeneration. The retained manifest supplies these original repository paths.
Newhaven's once-only starter grant is separately defined in
`data-global/lib/others/newhaven.lua:97-144`, and belongs to C19b.

## Accepted differences

The owner explicitly accepts historical starter compatibility: a character may
retain its recorded starter revision when that revision is registered for its
World. All other interpretation revisions, session generations, assignment,
authority and revision checks remain current. Existing characters retain their
items and receive no replacement kit. This exception applies only to starter
revision interpretation; it does not migrate possessions or weaken other fences.
Implementation: `crates/gameplay/durability/src/durability/character_authority.rs:113`
(`starter_revision_is_current_or_registered`) and
`apps/game-server/migrations/0982_newhaven_profession_starter_kits.sql:4`.
`character_progression.rs` retains exact current profile/ruleset/content checks
before applying that narrow starter predicate.

Newhaven kit evidence is retained Crystal revision
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/lib/others/newhaven.lua:3-95,97-144` (five profession tables,
base-profession mapping, once-only marker, equipment, backpack and quiver).
The production grant runs at first entry for an already selected profession and
retries after profession selection through the tutorial movement path. This is a
recorded integration difference from the donor portal's polling callback
(`data-global/scripts/quests/newhaven/movement_select_vocation.lua:3-34`).
The kit is gated by the character's recorded `newhaven-starter-r1` revision;
the integration base's legacy login loadout cannot also run for that revision.

## P01 donor rules

The retained charm mechanics source packet is
`tools/content-schema/charm-authoring/samples/charm-mechanics-sources-2026-10-01.json`;
its structured values are admitted by `rulesets/progression/charms/mechanics.json`.
The following Canary references use revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446`:

| Consumer | Donor evidence | Runtime rule |
|---|---|---|
| Carnage | `src/io/iobestiary.cpp:51-97`; `data/scripts/systems/bestiary_charms.lua:249-260` | Cardinal adjacent monsters, source maximum-health fraction with level cap, armour/resistance, no chain |
| Cripple | `src/io/iobestiary.cpp:105-109`; `src/creatures/combat/condition.cpp:2542-2553` | Native paralysis, 10 seconds, formula a=-1/b=0 |
| Numb | `src/io/iobestiary.cpp:149-153` | Same paralysis after an incoming hit |
| Adrenaline Burst | `src/io/iobestiary.cpp:143-147`; `src/creatures/combat/condition.cpp:2468-2471,2537-2563` | Native haste, 10 seconds, formula a=2.5/b=40 |
| Cleanse | `src/creatures/combat/combat.cpp:1031-1091`; `src/creatures/creature.cpp:1527-1548` | One negative condition removed, 11-second immunity |
| Fatal Hold | `src/io/iobestiary.cpp:177-181`; `src/creatures/monsters/monster.cpp:1456-1462,2155-2161` | Prevent fleeing for 30 seconds, preserving targeting and normal distance |
| Void Inversion | `src/game/game.cpp:9076-9095,9111-9113` | Invert the resolved mana drain atomically at its existing occurrence memo |
| Bless | `src/creatures/players/player.cpp:4084-4100,4098-4136` | Freeze the exact lethal-source assignment; reduce XP loss by 6/9/12 percent, retaining the accepted D58 build-preservation rule |

Source paths must be read through the retained source packet; donor code alone
is not claimed as independent official Tibia evidence. Existing native condition,
damage, receipt and progression owners retain authority and replay semantics.

The existing accepted difference D58 / NIGHT-W1 decision 19 is retained:
`crates/gameplay/domain/src/domain/death.rs:20` preserves all eight build families
on death. Bless reduces actual XP loss and retains that exact skill/magic state;
it does not introduce donor skill loss into a baseline which already preserves it.
The regression asserts the precise preserved build, the reduced XP loss, and
atomic rollback/replay of the captured lethal-source selection.

### Scavenge

Canary revision `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`data-otservbr-global/scripts/actions/tools/skinning.lua:200-208` binds the charm
to a monster's corpse or decayed corpse. Its file SHA-256 is
`ef230a2fcc109a93b688dd69746c15398274844694bfe0b6bceb782fffb447da`,
verified by fetching the exact immutable raw source and matching the retained
mechanics source packet.

The existing native base chances, spent bodies and yields match all 53 simple
rows of Crystal `data-crystal/scripts/actions/tools/skinning.lua` at revision
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`: minotaur 7 percent at lines 6-15;
dragon and dragon lord 5 percent at lines 211-231. The same revision's
`data-global` pack and the Canary pack use 25 percent. SRX-2 preserves the
existing native `data-crystal` base table and adds the relative charm bonus.

The current structured mechanics specify a positive relative success bonus of
60/90/120 percent. The donor scales the random range in the opposite direction;
that declining-chance defect is rejected in favour of the structured rule.
Chance is evaluated as an exact rational comparison, capped at certainty, with
no intermediate percentage rounding. Canonical minted creature provenance
supplies the race; shared corpse appearance IDs never establish race identity.

Item 10426 is a **piece of marble rock** in the pinned Canary Item XML, described
as a carving material. The separate marble reward branch at skinning.lua:217-242
is not a creature corpse and gets no Scavenge race or bonus. This is a scope
classification, not an excluded creature skinning rule or an early-return claim.

## Native client qualification

The existing inventory/equipment/container and charm progression protocol
families carry the data. No new capability, command ID or protocol message is
allocated. The immutable corpse race is selected by the typed, frozen Rust
creature reward owner, never by client input or a shared appearance ID. SQL
proves its fresh matching mint transaction and prevents later mutation; it does
not claim to reconstruct or attest a race from the opaque mint-intent hash. Native play tests exercise production decoders and the same panel
state/text consumed by the native windows. These tests do not claim a manual
Windows desktop play session.

## Reserved migrations

- 0982: Newhaven starter receipts and registered historical starter compatibility.
- 0983: Scavenge corpse provenance and atomic skinning proofs.

## Validation and exact filters

Revalidated on integration base `faf8ae8700d935bca612261730aca5466526389a`.
The removed C4 usable-item modules and migrations 0970/0971 remain absent.
Migrations 0982/0983 are retained. The base already supplies `MovementSpeech`;
SRX-2 uses that owner and retains its interleaved pending-owner regression,
without adding a duplicate command variant or changing casting rules.

`cargo fmt --all --check` and `git diff --check` passed. Clippy passed with
`--locked --offline --all-targets --all-features -- -D warnings` for all twelve
affected Rust packages: client, game-server, gameplay-combat, gameplay-content,
gameplay-dispatch, gameplay-domain, gameplay-durability, gameplay-items,
gameplay-monster-ai, gameplay-progression, gameplay-spell and session.
`cargo run --locked --offline -p oteryn-architecture-check -- workspace .`
passed (`workspace-boundaries: PASS`).

The dispatch selection passed 19 tests, zero failures, in 628.75 s, with the real
PostgreSQL 18.6 admin URL present. Exact filters:

- `newhaven_starter_`
- `srx2_charm_`
- `srx2_scavenge_production_use_minted_corpse_atomic_replay_postgres`
- `charm_progression_real_dispatch_unlocks_assigns_and_removes_for_the_gold_fee`
- `native_glyph_mana_drain_actual_mp_clamp_revision_no_hp_and_replay`
- `promotion_npc_postgres_dispatch_purchase_replay_bank_login_and_death`

All five Newhaven PostgreSQL bodies executed: rollback 46.675 s, registered
historical revision 46.431 s, first-entry grant 46.752 s, live publication 45.125 s,
and unregistered historical revision 45.182 s. All nine charm PostgreSQL cases
executed: Bless 83.75 s, Carnage 84.09 s, Cleanse 83.24 s, Fatal Hold 83.52 s,
Cripple 83.58 s, Adrenaline 83.52 s, Numb 83.50 s, Void Inversion 86.63 s,
and source-spell Parry 86.73 s. Scavenge's production-minted corpse case executed
in 47.205 s. The two native-glyph unit cases in that 19-test selection are not
represented as PostgreSQL proof. The same command selected zero durability
cases; its 0.00 s summary is not PostgreSQL evidence.

The native client selection passed all four tests, zero failures, in 0.09 s:

- `newhaven_starter_wire_data_reaches_native_equipment_backpack_and_quiver_panels`
- `srx2_charm_consumers_server_reply_reaches_native_panel`
- `skinning_decoded_stationary_corpse_updates_native_look_target`
- `movement_and_normal_speech_results_keep_their_pending_owner`

These exercise production wire decoders and the panel/scene state consumed by
native windows. They do not claim a manual Windows desktop session. No full
nextest, full dispatch unit suite or full PostgreSQL suite was run for this
rebase. Prior semantic REDs covered the missing kit, historical progression,
Carnage's adjacent HP 30-to-30 defect, source-spell Parry, Bless replay binding,
and the absent actual skinning use consumer; their repaired paths passed again.

The existing generated loot JSON keeps its key/entry order and changes only 63
added/63 removed lines. No newly added generated file exceeds 200 KB. Delivery
uses ordinary contextual file patches, including complete add/append records
for larger files, verified together with `git apply --index`; no patch script
or regeneration on the owner's computer is required. No new capability bit,
command ID or protocol message is allocated. No commit or push was made.
