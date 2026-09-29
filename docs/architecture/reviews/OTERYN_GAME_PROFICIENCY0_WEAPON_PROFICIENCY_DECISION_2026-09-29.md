# PROFICIENCY-0 Weapon Proficiency: content, persistence and wire

- Decision: `PROFICIENCY0-WEAPON-PROFICIENCY-V1`
- Status: **CANDIDATE**. Acceptance requires exact-head validation, independent review and
  protected integration. The wire part (§4.4) is a contract candidate that also needs owner
  acceptance, like every earlier state domain in `PROTOCOL_OTERYN_V1_REGISTRY.json`.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: #162 comment 5899469936 (`ARCHITECTURE_ESCALATION_REQUIRED`, PROFICIENCY-0, requested
  by the owner)
- Builds on: A13 (`OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md`, #1271) for the
  checkpoint pattern and the build chain; STANCE-0 §4 and DEATH-0 §3 for receipt kinds
- Migration, runtime, content and production authority: **NONE**. Each lane in §5 changes code
  under its own allocation and review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **PROF-CONTENT-1** (content lane, impl worker). Owned paths:
  `apps/game-server/src/content/project/v2.rs` (proficiency types), `content/proficiencies/**`,
  and the promotion tool beside `tools/content-census/stage_proficiencies.py`. It builds:
  - proficiency definitions as their own content records keyed by the CipSoft `ProficiencyId`
    (§4.1), promoted from `imports/cipsoft-staticdata/proficiencies/`;
  - the perk-code map, from Reference evidence; a definition with an unmapped perk code is not
    promoted (§4.1);
  - Item authoring references a definition by key; the inline `ProjectV2WeaponProficiencyProfile`
    is removed (§4.1).
  - Tests: every promoted definition resolves; an unmapped code is rejected; every weapon binding
    names a promoted definition or has none.
- **PROF-1** (durability lane, `oteryn-hard-worker`, persistence review; after CHAR-BUILD-1). Owned
  paths: the next free migration, `apps/game-server/src/durability/**`, its `*_postgres` tests. It
  builds the track table, the receipt kind with its line table, the indexed track chain, the writer
  and reconcile (§4.2-§4.3).
  - Tests: one revision per checkpoint covering every changed track; a row-only write fails; the
    track chain rejects a line whose before differs from that track's previous after; replay,
    conflict and a stale fence; one red run per guard branch.
- **PROF-2** (spell/combat lane). Progress accrual at kill credit, level-up, perk selection with the
  protection-zone rule, perk effects for mapped kinds, checkpoints through the PROF-1 writer (§4.3).
- **PROF-WIRE-1** (protocol lane, after owner acceptance of §4.4). The state domain, the command,
  the capability, the `.proto` and the registry rows; the native client view.
- Binding sections: §4.1-§4.5 of this document; A13 §4.2 and §4.5 (writer and checkpoint);
  DUR-02 rule 2; GAME-CHAR-01 Stage B decision 9.

## 1. Question

The owner asked for Weapon Proficiency. Implementation cannot start until three things are fixed:

- the canonical content format and how Items reference it;
- who stores per-Character progress and selected perks, and how durably;
- the client wire for viewing progress and choosing perks.

## 2. Owner decisions

None new. The owner's request (#162 5899469936) puts Weapon Proficiency in scope. GAME-CHAR-01 Stage B
decision 9 already places "per-weapon Weapon Proficiency Progress and player-owned proficiency
state" in the Character domain.

## 3. Facts

**PROVEN** (main `d4cb72ee`)

- `domain/progression.rs` rejects proficiency mutations (`UnsupportedSkillOrProficiencyMutation`).
  No migration stores proficiency.
- `content/project/v2.rs` carries an inline `ProjectV2WeaponProficiencyProfile` on each Item
  (`levels[{level, perks: [ProjectV2AugmentBinding]}]`, optional `shaping`).
- #1283 staged the 15.30 client source (`imports/cipsoft-staticdata/proficiencies/`,
  `imports/cipsoft-staticdata/weapon-proficiency-bindings/`):
  - 443 definitions with `ProficiencyId`, `Name`, `Version` and `Levels[{Perks}]`;
  - eight closed perk key sets, whose `Type`, `SkillId`, `AugmentType`, `ElementId` and
    `DamageType` codes are kept raw and not interpreted;
  - 666 client objects bind to 430 distinct definitions; up to 23 objects share one definition.
- The Tibia manual (`docs/reference/tibia-manual/combat.md` §5.3.4, CipSoft official):
  - progress is earned by defeating monsters while the weapon is equipped, with Bestiary-style
    credit for every damage-contributing player;
  - a kill gives 1 to 175 points, and a boss up to 1,000, scaled by tier;
  - there are 1 to 7 levels, from 1,750 to 20,000,000 progress;
  - each level offers 1 to 3 perks, and only one is active per level. An unfilled slot is picked
    freely; changing a chosen perk needs a protection zone;
  - perk modification, rank, reshape, clear, the Lunar Ascension Orb and catalysts are described,
    but their dust costs are not;
  - Mastery (2 more levels' worth) is cosmetic;
  - progress is per Character and not transferable.
- The item key is the Tibia client id (A12, `oteryn:item.tibia.i<id>`).
- `PROTOCOL_OTERYN_V1_REGISTRY.json` has three state domains (1-3). Each was allocated with an
  owner acceptance. Capability ids and command types are assignable.
- DUR-02 rule 2: every Character semantic transaction advances one `CharacterRevision`.
- A13 (#1271) fixes the checkpoint pattern for training progress. The #1271 review found that the
  `0017` consistency guard reads the whole chain on every row event.

**UNKNOWN**, to be settled from Reference evidence by the lanes in §5:

- the meaning of the raw perk codes;
- the creature tier table for 1-175 points, and multi-player weighting;
- whether two weapons sharing one definition also share progress (§4.2 "Track key");
- dust costs, modification odds, catalysts and the Lunar Ascension Orb.

## 4. Decision

### 4.1 Content

- **Definitions.** A proficiency definition is its own content record, keyed by the CipSoft
  `ProficiencyId` (`oteryn:proficiency.tibia.p<id>`, in the A12 pattern). It holds the levels, each
  level's perks, and the progress threshold of each level and of Mastery.
- **Items.** Item authoring gains `proficiency: Option<ProficiencyKey>`, filled from the weapon
  binding (flags field 61). The inline `ProjectV2WeaponProficiencyProfile` and
  `ProjectV2PerkShaping` are removed; `ProjectV2ProficiencyLevel` moves into the definition.
- **Perks.** A perk is a typed record of one closed Oteryn perk kind, mapped from the source key
  set and codes by a table that cites Reference evidence. A definition with any unmapped code stays
  staged and is not promoted, and its weapons have no proficiency until it is. Nothing guesses a
  code.
- **Thresholds.** The per-level thresholds come from Reference evidence, cited by the content lane.
  They are content, not Character state, so a content revision can correct them.

### 4.2 Persistence

- **Track key.** One track per (Character, weapon Item key), because the manual describes a
  per-weapon tree that trains while "that weapon" is equipped. If Reference evidence shows that
  weapons sharing a definition share progress, that superseding evidence re-keys the track to
  (Character, `ProficiencyKey`) before PROF-1 ships.
- **Table.** `game_character_proficiency` has one row per track:
  - `character_id` and `item_key`;
  - `progress` (cumulative and non-decreasing), and the selected perk index per level;
  - `committed_character_revision` and `last_proficiency_occurrence_id`.

  The level and Mastery are derived from `progress` and the definition's thresholds, and are not
  stored. No row means progress 0 and no selection. The row is created by its first receipt line,
  never deleted, and truncate is rejected.
- **Receipt kind.** `game_character_proficiency_receipts` is one receipt per revision, in the
  `0009` chain (DUR-02 rule 2, one receipt per revision):
  - `proficiency_occurrence_id` (UUIDv7), `character_id`, the revision pair, and level and total
    experience before and after, which are equal;
  - a cause: `training` (progress only) or `perk_selection` (one track's selection, progress
    equal);
  - `command_binding`, `policy_digest`, the revision fields of an XP receipt and `committed_at`.
- **Lines.** `game_character_proficiency_receipt_lines`: one line per changed track in that
  receipt, keyed by (occurrence, `item_key`), with progress and the selection before and after.
  - A `training` receipt has 1 to N lines. N is the number of tracks changed since the previous
    checkpoint, bounded by the weapons with a proficiency in the active content.
  - A `perk_selection` receipt has exactly one line.
  - A no-op line is rejected.
- **Track chain.** Each line's before equals the after of the latest earlier line for the same
  track, or (0, no selection). The guard finds that predecessor through an index on
  (`character_id`, `item_key`, `committed_character_revision`), and checks only the current
  revision's lines. It never scans the whole chain, so its cost does not grow with the chain (the
  #1271 review concern).
- **Row guard.** Each row equals the latest line for its track. The row has a deferred constraint
  trigger on insert, update and delete (delete rejected), and so do the receipt and its lines on
  insert. The revision-1 branch rejects every proficiency relation.
- **Shared guard.** The kind joins the guard-rewrite chain after CHAR-BUILD-1; the #162 lease
  orders it against H-1. It counts proficiency receipts in the `revision − 1` total, and the
  state guard's equal-XP branch admits it.

### 4.3 Writer and checkpoints

- **Writer.** `commit_character_proficiency` and `reconcile_character_proficiency` mirror
  `commit_character_build` (A13 §4.2 "Writer"): per-occurrence advisory lock, replay lookup,
  recovery fence, admission locks, session-generation fence, `character_root` row lock;
  replay-or-conflict with the binding compared before replay; a stale fence writes nothing; one
  ordered commit queue per session.
- **Accrual.** Progress accrues in the live session at kill credit for the equipped weapon, from
  the Reference point table.
- **Checkpoints.** One `training` receipt covers every changed track:
  - at a level-up, which is always durable, and the level's perk slot opens only after that
    receipt commits;
  - at logout;
  - at a checkpoint of at most 60 seconds, the A13 §4.5 bound, skipped when nothing changed.

  A crash may lose at most one checkpoint of progress, never a level.
- **Perk selection** is a command. It commits one `perk_selection` receipt, and the effect applies
  only after that receipt commits. An unfilled slot is picked anywhere. Changing a chosen perk
  needs a protection zone; the runtime checks the zone, and the receipt records the selection.
- **Growth.** Receipts grow by at most about one per minute per trained Character, beside the A13
  mana checkpoints. PROF-2 measures receipts, lines and guard latency against chain length, and
  reports before V1 ships. If the measurement shows a need, a later decision may combine the A13
  and proficiency checkpoints into one receipt; this decision does not.

### 4.4 Wire (contract candidate, owner acceptance required)

- **State domain.** One new own-actor state domain, `ACTOR_PROFICIENCY`, owned by the current
  ChannelRuntime:
  - a snapshot of every track with progress greater than 0: `item_key`, progress and the
    selections;
  - a delta per changed track after its receipt commits.

  The client derives levels from the definitions in its content. Its id, the message types and
  `max_payload_bytes` are allocated in the registry by PROF-WIRE-1, with bounds taken from the
  promoted content count, not chosen here.
- **Command.** One command, `PROFICIENCY_SELECT_PERK {item_key, level, perk_index}`, with a
  result: accepted, rejected with a reason (not unlocked, not in a protection zone, unknown track
  or perk), or a stale-revision conflict.
- **Gating.** Both are behind a new capability. A client that does not advertise it receives
  neither the domain nor the command result, and its server-side progress still accrues. The
  pattern follows `actor_spell_v1`.

### 4.5 Deferred within V1

Perk modification (reroll, rank, reshape, clear), the Lunar Ascension Orb and catalysts spend value
(dust, items), and their costs are UNKNOWN (§3). They get their own decision (PROFICIENCY-1) when
Reference evidence exists. This is sequencing, not a scope cut: they stay in V1 scope. The receipt
kind leaves room for a `modification` cause. Mastery is cosmetic and derived from `progress`.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| PROF-CONTENT-1 | Definitions, perk map, Item reference, removal of the inline profile | this decision; ITEM-ID-1 keys |
| PROF-1 | Migration, writer, reconcile, guards (§4.2-§4.3). Persistence review. | this decision; CHAR-BUILD-1 |
| PROF-2 | Accrual, level-up, perk selection, perk effects, checkpoints, measurement | PROF-CONTENT-1, PROF-1 |
| PROF-WIRE-1 | Registry rows, `.proto`, capability, client view | owner acceptance of §4.4; PROF-2 |
| PROFICIENCY-1 | Modification, rank, catalysts (decision) | Reference evidence for costs |

## 6. Rejected options

- **Keep the inline profile per Item.** It would duplicate 430 shared definitions across 666
  weapons, and a correction would have to touch every copy.
- **A receipt per kill.** It adds a durable write to every kill and grows the chain without bound.
- **A store outside the CharacterRevision chain.** Proficiency is Character progression
  (GAME-CHAR-01 decision 9), and DUR-02 rule 2 puts every semantic change in the chain. An
  exception would change an owner baseline.
- **A receipt per track per checkpoint.** Several trained weapons would each take a revision. One
  receipt with lines takes one.
- **Stored level.** A level derived from `progress` stays correct when content thresholds are
  corrected.

## 7. Decision test

- **Must decide now:** YES. The owner asked for the system, and no lane can start without the
  format, the store and the wire.
- **Minimum sufficient:** one definition record, one track table, one receipt kind with lines, and
  one state domain with one command.
- **Superseding evidence:** a Reference rule for shared progress (§4.2), or for perk codes and
  point tables that contradicts §4.1-§4.3.
- **Deliberately not decided:** modification costs and rules (§4.5), perk effect numbers, and the
  point table.

## 8. Handback

```yaml
result: RESOLVED
source_escalation: "#162 5899469936 (PROFICIENCY-0)"
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # PROF-CONTENT-1 now; PROF-1 after CHAR-BUILD-1; PROF-WIRE-1 after owner acceptance of §4.4
required_fresh_allocation: true
required_independent_review: "this decision (persistence); PROF-1 persistence review"
owner_acceptance_required: "§4.4 wire contract candidate (state domain + command + capability)"
remaining_unknowns: [perk code meanings, point table, shared-progress rule, modification costs]
next_action: "#162 validates this exact head, routes the independent review and the §4.4 owner acceptance, integrates it, then allocates PROF-CONTENT-1."
```

## 9. Before-freeze checklist

1. Contract amendments: none of an accepted contract. The wire in §4.4 is a candidate for the
   registry, and its rows are written with the allocation.
2. Serialization: `character_root` row lock, session-generation fence, one ordered queue per
   session; the losing writer writes nothing (§4.3).
3. Restart: rows, receipts and lines hold every committed fact. At most one checkpoint of progress
   is lost, never a level or a selection (§4.3).
4. Typed references: lines are keyed by (occurrence, `item_key`); `last_proficiency_occurrence_id`
   is typed by (`character_id`, `committed_character_revision`) and the receipt kind; Items name a
   `ProficiencyKey`.
5. Wire: a new capability gates the domain and the command; older clients receive neither (§4.4).
6. Split work: one checkpoint is one revision covering every changed track (§4.2 "Lines").
