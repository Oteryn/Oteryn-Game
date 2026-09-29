# PROFICIENCY-0 Weapon Proficiency: content, persistence and wire

- Decision: `PROFICIENCY0-WEAPON-PROFICIENCY-V1`
- Status: **CANDIDATE**. Acceptance requires exact-head validation, independent review and
  protected integration. The wire part (§4.4) is a contract candidate that also needs owner
  acceptance, like every earlier state domain in `PROTOCOL_OTERYN_V1_REGISTRY.json`.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: #162 comment 5899469936 (`ARCHITECTURE_ESCALATION_REQUIRED`, PROFICIENCY-0, requested
  by the owner)
- Builds on: A13 (`OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md`, #1271, still a
  candidate) for the writer and checkpoint pattern; STANCE-0 §4 and DEATH-0 §3 for receipt kinds;
  GAME-CHAR-01 Stage B decisions 9 and 10. This decision is accepted only after A13 is.
- Migration, runtime, content and production authority: **NONE**. Each lane in §5 changes code
  under its own allocation and review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **PROF-CONTENT-1** (content lane, impl worker). Owned paths:
  - `content/proficiencies/**`, `content/project.json`, `content/manifest.json`,
    `content/content.lock.json`;
  - `apps/game-server/src/content/project/v2.rs`, and the `DefinitionFamily` in
    `apps/game-server/src/content/reference_playable.rs`;
  - `apps/game-server/tests/content_world_project_v2.rs`;
  - `tools/content-schema/item-authoring/**`, and a promotion tool beside
    `tools/content-census/stage_proficiencies.py`.

  It builds:
  - `Proficiency` definitions from the 15.30 client source, with the perk-code map (§4.1);
  - the Item `profile_binding` as the only form. The inline levels and shaping are removed from
    both schemas (§4.1).
  - Tests: every promoted definition resolves; an unmapped code is rejected; the coverage report
    lists the promoted and staged definitions and weapons.
- **PROF-1** (durability lane, `oteryn-hard-worker`, persistence review). It comes after
  CHAR-BUILD-1 and H-1, and carries every receipt kind. Owned paths: its migration,
  `apps/game-server/src/durability/**`, and its `*_postgres` tests. It builds:
  - the track table, the receipt kind and its lines;
  - the per-track guard and the shared chain entry;
  - the writer, reconcile, and the admission load (§4.2-§4.3).
  - Tests:
    - one revision per checkpoint, covering every changed track;
    - a row-only write fails, also at revision 1;
    - a line added to an older receipt fails;
    - the track chain rejects a wrong before value;
    - the line count per cause is checked;
    - replay, conflict and a stale fence;
    - one red run per guard branch.
- **PROF-2** (combat lane). Accrual at kill credit, level-up, perk selection and clearing with the
  protection-zone rule, the perk effects of mapped kinds, checkpoints, and measurement (§4.3).
- **PROF-WIRE-1** (protocol lane, after owner acceptance of §4.4). The first registry capability,
  the state domain, the command, the `.proto` and the client view.
- Binding sections: §4.1-§4.5 of this document; A13 §4.2 "Writer" and §4.5; DUR-02 rule 2;
  GAME-CHAR-01 Stage B decision 9 rules 3 and 6, and decision 10.

## 1. Question

The owner asked for Weapon Proficiency. Implementation cannot start until three things are fixed:

- the canonical content format and how Items reference it;
- who stores per-Character progress and selected perks, and how durably;
- the client wire for viewing progress and choosing perks.

## 2. Owner decisions

None new. The owner's request (#162 5899469936) puts Weapon Proficiency in scope. GAME-CHAR-01 Stage
B decision 9 already places per-weapon proficiency progress in the Character domain. Its rule 3
gives trees, thresholds and perks to content. Its rule 6 requires versioned definition references.

## 3. Facts

**PROVEN** (main `50d75c6`)

- `domain/progression.rs` rejects proficiency mutations (`UnsupportedSkillOrProficiencyMutation`).
  No migration stores proficiency.
- `content/project/v2.rs` carries an inline `ProjectV2WeaponProficiencyProfile` on each Item.
- The formal item-authoring schema (`tools/content-schema/item-authoring/item.schema.json`,
  candidate 4) already has `ProficiencyRef {family: "Proficiency", key, revision}` and
  `proficiency.profile_binding`, next to inline `levels` and `shaping`. The README describes a
  crosswalk rule: a numeric source id stays provenance until a pinned crosswalk binds it, and
  admission needs Canary and Crystal to corroborate.
- #1283 staged the 15.30 client source (`imports/cipsoft-staticdata/proficiencies/`,
  `.../weapon-proficiency-bindings/`):
  - 443 definitions with `ProficiencyId`, `Name`, `Version` (1-13) and `Levels[{Perks}]`;
  - eight closed perk key sets, whose codes are raw;
  - 666 client objects bind to 430 definitions. The binding field (flags field 61) is inferred,
    per the bindings README.
- The Tibia manual (`docs/reference/tibia-manual/combat.md` §5.3.4, CipSoft official):
  - progress comes from monsters defeated while that weapon is equipped, with Bestiary-style
    credit;
  - a kill gives 1 to 175 points, and a boss up to 1,000;
  - there are 1 to 7 levels;
  - level 1 needs 1,750 and level 7 needs 20,000,000;
  - each level offers 1 to 3 perks, and only one is active per level;
  - an unfilled slot is picked freely; changing or removing a chosen perk needs a protection zone;
  - modification spends dust;
  - Mastery is cosmetic;
  - progress is per Character.
- The item key is the Tibia client id; an alias resolves to one canonical key (A12).
- `PROTOCOL_OTERYN_V1_REGISTRY.json` has no capability (`"capabilities": []`) and three state
  domains, each owner-accepted.
- DUR-02 rule 2: every Character semantic transaction advances one `CharacterRevision`, and may
  change several child relations in it.
- The `0017` consistency guard counts and self-joins the whole chain on every fire (#1271 review).

**UNKNOWN** (hard parity gates under Stage B decision 10):

- the meaning of the raw perk codes;
- the point table (creature tiers, bosses) and multi-player weighting;
- the thresholds of levels 2-6, of trees shorter than 7 levels, and of Mastery;
- whether weapons sharing a definition share progress;
- dust costs, modification odds, catalysts and the Lunar Ascension Orb.

## 4. Decision

### 4.1 Content

- **Definitions.** `Proficiency` is a content definition family. A definition is keyed
  `oteryn:proficiency.tibia.p<ProficiencyId>`, in the A12 pattern, and carries a revision. It holds
  the levels, each level's perks, and each level's threshold.
  - The definition is promoted from the pinned 15.30 client source, which is CipSoft's own data.
    That source is enough to admit it. The Canary and Crystal corroboration in the item-authoring
    crosswalk rule becomes optional evidence for proficiency. This amends that rule, in
    `tools/content-schema/item-authoring/README.md` (PROF-CONTENT-1).
  - Thresholds and the point table enter only from Reference evidence. Until then a definition
    carries the evidenced ones, and the rest are UNKNOWN parity gates (§4.5).
- **Items.** `proficiency` has one form, `profile_binding: ProficiencyRef` (family, key,
  revision), filled from the weapon binding (flags field 61, inferred). The inline `levels` and
  `shaping` are removed from `item.schema.json` and from `ProjectV2WeaponProficiencyProfile`.
  `ProjectV2ProficiencyLevel` moves into the definition. PROF-CONTENT-1 updates the fixtures,
  dispositions and tests that use them.
- **Perks.** A perk is a typed record of one closed Oteryn perk kind. It is mapped from the
  source key set and codes by a table that cites Reference evidence.
  - A definition with an unmapped code stays staged and is not promoted.
  - A weapon whose definition is not promoted has no proficiency. Nothing accrues for it, and
    nothing is stored.
  - PROF-CONTENT-1 reports this coverage.
- **Revisions (rule 6).** A new definition revision is compatible when it keeps the level count
  and each level's perk order. It may change values and thresholds.
  - A compatible revision applies without a write. The track's stored revision advances with its
    next line.
  - Anything else is incompatible. The content revision declares the migration of the selections
    (keep, remap or clear, per level). The owning session writes one `migration` receipt for the
    Character's affected tracks before it first uses any of them under the new revision, in its
    own transaction after admission. Until that receipt commits, the tracks accrue nothing and
    their perks are inactive.

### 4.2 Persistence

- **Track key.** One track per (Character, canonical weapon Item key), after A12 alias resolution.
  The manual describes a per-weapon tree that trains while "that weapon" is equipped. If Reference
  evidence shows that weapons sharing a definition share progress, that superseding evidence
  re-keys the track to (Character, definition key) before PROF-1 ships.
- **Table.** `game_character_proficiency` has one row per track:
  - `character_id`, `item_key`, and the definition key and revision the track follows (rule 6);
  - `progress` (BIGINT, cumulative, non-decreasing);
  - `selections SMALLINT[]`, one entry per level: the perk index, or NULL for an unfilled slot;
    length at most 7;
  - `committed_character_revision` and `last_proficiency_occurrence_id`.

  No row means progress 0 and no selection. The row is created by its first line, never deleted,
  and truncate is rejected.
  - The level and Mastery are derived from `progress` and the definition's thresholds, and are
    not stored.
  - A selection at a level above the derived level stays stored but inactive. That happens when a
    compatible revision raises a threshold.
- **Receipt kind.** `game_character_proficiency_receipts` is one receipt per revision in the `0009`
  chain. It holds:
  - `proficiency_occurrence_id` (UUIDv7), `character_id`, and the revision pair;
  - level and total experience before and after, which are equal;
  - a cause: `training`, `perk_selection` or `migration`;
  - `command_binding`, `policy_digest`, the revision fields of an XP receipt, and `committed_at`.
- **Lines.** `game_character_proficiency_receipt_lines` has one line per changed track. Each line
  holds:
  - `proficiency_occurrence_id`, `character_id`, `committed_character_revision` and `item_key`,
    with a composite FK (occurrence, character, revision) to its receipt;
  - the definition key and revision, before and after;
  - `progress` and `selections`, before and after.

  CHECKs per cause:
  - `training`: progress strictly increases; selections and definition are unchanged;
  - `perk_selection`: progress and definition are unchanged; exactly one level's entry changes
    (set, change or clear);
  - `migration`: progress is unchanged, the definition revision changes, and the selections
    follow the declared migration.

  The receipt trigger checks the line count: 1 to N for `training` and `migration`, and exactly 1
  for `perk_selection`. N is bounded by the weapons with a proficiency in the active content.
  Lines are immutable and cannot be truncated.
- **Guards.** There are two functions, so that a checkpoint does not run the whole-chain check once
  per line:
  - the **shared chain check** (the `0017` consistency guard, extended) runs only from the receipt
    and root triggers. It counts proficiency receipts in the `revision − 1` total. Its revision-1
    branch rejects every proficiency relation.
  - the **per-track check** runs from the line and row triggers. It accepts a line only when its
    receipt's committed revision equals the current root revision. It then finds the track's
    previous line through an index on (`character_id`, `item_key`,
    `committed_character_revision`), and checks that the line's before values equal that line's
    after values, or (0, none, the first definition revision). The row must equal the track's
    latest line. The row has a deferred trigger on insert, update and delete (delete rejected).

  The per-track check's cost does not depend on chain length. The shared chain check still reads
  the whole chain once per revision. PROF-2 measures it (§4.3), as A13 W2b does.
- **Shared guard order.** PROF-1 comes after CHAR-BUILD-1 and H-1 in the guard-rewrite chain, and
  carries every receipt kind so far. The state guard's equal-XP branch admits it.

### 4.3 Writer and checkpoints

- **Writer.** `commit_character_proficiency` and `reconcile_character_proficiency` mirror
  `commit_character_build` (A13 §4.2 "Writer"):
  - the per-occurrence advisory lock, replay lookup, recovery fence, admission locks,
    session-generation fence and `character_root` row lock;
  - replay-or-conflict, with the binding compared before replay;
  - a stale fence writes nothing;
  - one ordered commit queue per session;
  - an initialized progression row is required.
- **Losing writer.**
  - `training` keeps its pending progress as a per-track delta. It applies the delta to the latest
    committed values and retries at the next revision with a new occurrence.
  - A stale `perk_selection` is re-validated against the committed row. If it is still valid, it
    retries once with a new occurrence; otherwise the command result is rejected.
  - The occurrence of a command is derived from its `CommandId` by the rule STANCE-1 sets for
    toggles, so a client retry replays.
- **Admission and recovery.**
  - Admission loads the rows, and writes nothing.
  - Pending progress belongs to the live session. A new `GameSessionId` after a recovery starts
    from the committed rows. At most one checkpoint of progress is lost.
- **Accrual.** Progress accrues in the live session at kill credit for the equipped weapon.
- **Checkpoints.** One `training` receipt covers every changed track:
  - at a level-up, which is always durable. The level's perk slot opens only after that receipt
    commits;
  - at logout;
  - at a checkpoint of at most 60 seconds (the A13 §4.5 bound), skipped when nothing changed.
- **Perk selection and clearing** are commands. Each commits one `perk_selection` receipt, and the
  effect applies only after it commits.
  - An unfilled slot may be set anywhere.
  - Changing or clearing a set slot needs a protection zone. The runtime checks the zone; the
    receipt records the change.
- **Growth.** Receipts grow by at most about one per minute per trained Character, beside the A13
  mana checkpoints. PROF-2 measures receipts, lines and guard latency against chain length, and
  reports before V1 ships. A later decision may merge the two checkpoints into one; this decision
  does not.

### 4.4 Wire (contract candidate, owner acceptance required)

- **Capability.** This is the first registry capability, and PROF-WIRE-1 defines the registry row
  shape: id, name, owner decision, and the message types it gates. A session that did not
  negotiate it receives neither the domain nor any command result, because it cannot send the
  command. An unnegotiated `PROFICIENCY_SELECT_PERK` is answered with `CAPABILITY_MISMATCH`
  (1007). Capabilities are fixed for the session; a resume renegotiates them like a new session.
  Progress accrues server-side regardless.
- **State domain.** `ACTOR_PROFICIENCY` is an own-actor domain owned by the current ChannelRuntime:
  - a snapshot of every track with progress > 0: numeric Tibia item id (varint), definition
    revision, progress (varint) and the selections;
  - a delta per changed track after its receipt commits. The client sees progress at checkpoint
    granularity, up to 60 seconds late. This is a deliberate choice: durable progress only, as
    with vitals from committed state.
  - Both name the content generation, so the client derives levels from the same definitions.
  - Bound, for PROF-WIRE-1 to confirm: 666 tracks × (item id ≤ 3 B + revision ≤ 2 B + progress
    ≤ 5 B + 7 selections ≤ 7 B), about 12 KB per snapshot and 17 B per delta.
- **Command.** `PROFICIENCY_SELECT_PERK {item_id, level, perk_index | clear, expected_revision}`.
  `expected_revision` is the track's committed revision as the client last saw it. The result is
  one of:
  - accepted;
  - rejected: not unlocked, not in a protection zone, unknown track or perk;
  - a stale-revision conflict, when `expected_revision` differs.
- The state domain and command ids, the message types and `max_payload_bytes` are allocated in
  the registry by PROF-WIRE-1.

### 4.5 Parity gates and deferred work

- **Parity gates (Stage B decision 10).** The point table, the thresholds of levels 2-6 and of
  Mastery, and multi-player weighting are UNKNOWN. PROF-2 may build them behind a versioned
  policy, but proficiency is not enabled in production, nor claimed as Reference behaviour, until
  each is evidenced or owner-accepted as a `DECLARED_DIFFERENCE`.
- **Deferred.** Perk modification (reroll, rank, reshape), the Lunar Ascension Orb and catalysts
  spend value (dust, items) under GAME-ITEM-01/DUR-03 (rule 4), and their costs are UNKNOWN. They
  get their own decision (PROFICIENCY-1). This is sequencing, not a scope cut. Mastery is
  cosmetic and derived.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| PROF-CONTENT-1 | Definitions, perk map, `profile_binding` only, removal of the inline profile, the crosswalk-rule amendment, coverage report | this decision; ITEM-ID-1 keys |
| PROF-1 | Migration, writer, reconcile, guards, admission load (§4.2-§4.3). Persistence review. | this decision; A13 accepted; CHAR-BUILD-1; H-1 |
| PROF-2 | Accrual, level-up, selection and clearing, perk effects, checkpoints, measurement | PROF-CONTENT-1, PROF-1 |
| PROF-WIRE-1 | Capability row shape, domain, command, `.proto`, client view | owner acceptance of §4.4; PROF-2 |
| PROFICIENCY-1 | Modification, rank, catalysts (decision) | Reference evidence for costs |

## 6. Rejected options

- **Keep the inline profile per Item.** It would duplicate 430 shared definitions across 666
  weapons, and a correction would have to touch every copy.
- **A receipt per kill.** It adds a durable write to every kill and grows the chain without bound.
- **A store outside the CharacterRevision chain.** Proficiency is Character progression (Stage B
  decision 9), and DUR-02 rule 2 puts every semantic change in the chain. An exception would change
  an owner baseline.
- **A receipt per track per checkpoint.** Several trained weapons would each take a revision. One
  receipt with lines takes one (DUR-02 rule 2 allows several child relations in one revision).
- **Stored level.** A derived level stays correct when a compatible revision corrects thresholds.
- **Live, non-durable progress on the wire.** It adds a second source of truth for the client for
  a display gain only.

## 7. Decision test

- **Must decide now:** YES. The owner asked for the system, and no lane can start without the
  format, the store and the wire.
- **Minimum sufficient:** one definition family, one track table, one receipt kind with lines, and
  one capability-gated domain with one command.
- **Superseding evidence:** a Reference rule for shared progress (§4.2), or for perk codes and
  point tables that contradicts §4.1-§4.3.
- **Deliberately not decided:** modification (§4.5), perk effect numbers, and the point table.

## 8. Handback

```yaml
result: RESOLVED
source_escalation: "#162 5899469936 (PROFICIENCY-0)"
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # PROF-CONTENT-1 now; PROF-1 after A13 acceptance, CHAR-BUILD-1 and H-1; PROF-WIRE-1 after owner acceptance of §4.4
required_fresh_allocation: true
required_independent_review: "this decision (persistence, wire candidate); PROF-1 persistence review"
owner_acceptance_required: "§4.4 wire contract candidate (first capability, state domain, command)"
remaining_unknowns: [perk code meanings, point table, thresholds L2-L6 and Mastery, shared-progress rule, modification costs]
next_action: "#162 validates this exact head, routes the independent review and the §4.4 owner acceptance, integrates it, then allocates PROF-CONTENT-1."
```

## 9. Before-freeze checklist

1. Contract amendments:
   - Stage B decision 9 rule 6 is complied with, not amended: rows and lines store the definition
     key and revision, and incompatible revisions migrate explicitly (§4.1-§4.2).
   - The item-authoring formal schema and its crosswalk rule change. PROF-CONTENT-1 owns them as
     tool artifacts with tests (§4.1).
   - The registry's first capability and its row shape are part of the §4.4 candidate.
   - A13 is a dependency; this decision is accepted after it.
2. Serialization: `character_root` row lock, session-generation fence, one ordered queue per
   session. The losing writer writes nothing and retries as §4.3 "Losing writer" says.
3. Restart: rows, receipts and lines hold every committed fact. Admission loads the rows. At most
   one checkpoint of progress is lost, never a level or a selection (§4.3).
4. Typed references: lines carry (occurrence, character, revision, `item_key`) with a composite FK;
   rows and lines name `ProficiencyRef` (family, key, revision); Items use
   `profile_binding: ProficiencyRef`.
5. Wire: a new capability gates the domain and the command; an unnegotiated command gets 1007
   (§4.4).
6. Split work: one checkpoint is one revision covering every changed track; the line count is
   checked at commit (§4.2).
7. Self-review: `oteryn-hard-worker`, read-only, on the complete draft. Its material findings
   (whole-chain cost per line, late lines, rule 6, derived-level corrections, line keys, owned paths,
   amendments) are fixed in §4.1-§4.4 and the brief.
