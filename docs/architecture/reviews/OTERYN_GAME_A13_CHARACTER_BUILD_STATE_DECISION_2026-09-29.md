# A13 Character build state (vocation and magic level)

- Decision: `A13-CHARACTER-BUILD-STATE-V1`
- Status: **CANDIDATE with owner decisions D150-D151 (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: #162 comment 5896414182 (`ARCHITECTURE_ESCALATION_REQUIRED`, SPELL-CASTER-FACTS) and
  the owner question in 5896342127
- Ruling posted: #162 comment 5896480875
- Repaired after the independent review of `d4e97ce` (#1265 5897183488; #162 A13-RECEIPT-CHAIN,
  5897202372), then by the successor architect after the repaired head `ee21804d` was checked
  against `0016`, the STANCE-0 migration `0017` (#1270) and the before-freeze checklist (§9)
- Decides: the "Magic-level training" item in
  `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §10
- Migration, runtime, content and production authority: **NONE**. Each lane in §5 changes code
  under its own allocation and review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **CHAR-BUILD-1** (durability lane, `oteryn-hard-worker`, persistence review). Owned paths: the
  next free migration, `apps/game-server/src/durability/**` for the writer and reconcile, the
  admission load, and its `*_postgres` tests. It builds:
  - `game_character_build_state` and `game_character_build_receipts` (§4.1-§4.2). No row means
    (`none`, 0, 0). No creation insert, no backfill.
  - The build kind in the shared `0009` guards. The #162 lease orders the guard rewrites STANCE-0
    (`0017`, #1270), then CHAR-BUILD-1, then H-1 (spell contract §8.2), so CHAR-BUILD-1 starts
    from the last merged rewrite and carries XP, death, stance and build (§4.2 "Shared guard").
  - `commit_character_build` and `reconcile_character_build` (§4.2 "Writer").
  - The admission load into the live Character (§4.1).
  - Tests: a row-only write fails, also at revision 1; a revision-1 Character has no build
    receipt; each cause rejects the wrong direction; the chain holds
    across XP, death, stance and build; the stance chain counts only a build receipt whose stance
    fields differ (§4.2 "Stance fields"); replay, conflict and a stale fence; one red run per
    guard branch.
- **W2b** (spell lane, after CHAR-BUILD-1). `CasterState` reads build state, adds
  `Vocation::None` (§4.3), accumulates training and commits checkpoints through
  `commit_character_build` (§4.5). The formula is cited from Reference evidence. It measures
  receipt growth (§4.5 "Growth").
- **DAWNPORT-1** (content lane). The choice interaction calls the vocation writer with the combined
  stance fields (§4.4).
- **DEATH ML loss** (DEATH lane, after CHAR-BUILD-1 and DEATH-1). The composite death transaction
  (§4.6), including retry and reconcile of its revisions.
- Binding sections: §4.1-§4.6 of this document; STANCE-0 §4.3 and §4.6; DEATH-0 §3.1-§3.2 and
  §3.5, with the one refinement in §4.6 "One fence".

## 1. Question

`CasterState` needs a Character's vocation and magic level. Without them, the SPELL-D4 cast gate
stays closed, so every production cast is rejected (#1263). Three things must be settled:

- which store owns these facts;
- whether they join the CharacterRevision receipt chain;
- how a Character acquires them in V1.

## 2. Owner decisions

The owner confirmed these directly in this session on 2026-09-29.

| # | Decision | Owner choice |
|---|---|---|
| D150 | Vocation is acquired as in Global. A Character starts without a vocation and chooses it on Dawnport before leaving at level 8. There is no choice at creation and no dev-only profile. | "Jak Global: Dawnport" |
| D151 | Magic-level training from mana spent ships in V1, as in Global. It is not deferred. | "Wszystko od razu" |

## 3. Facts

**PROVEN** (main `48de3868`, #162 5896414182)

- `0009` `game_character_progression_state` stores only `level` and `total_experience`. No
  migration from `0001` to `0015` stores a vocation, magic level or mana spent.
- The Character aggregate and game-domain mutations are Game-owned. Platform may only project
  class or vocation (`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY`).
- `CasterState` (`apps/game-server/src/spell/mod.rs`) reads `vocation` and `magic_level`. The
  `Vocation` enum has no "none" value.
- The spell cast contract §10 left magic-level training undecided.
- DEATH-0 (#1264, migration `0016`), STANCE-0 and H-1 all rewrite the same `0009` guard functions.
- At root revision 1 the consistency guard rejects every receipt of any kind (`0009`; `0016`;
  `0017` on #1270 head `ff7ba430`, "initial Character progression is inconsistent"). For
  revision > 1 the receipts of all kinds total `revision − 1`.
- The D88 progression initializer inserts the progression row without a receipt and without
  advancing `CharacterRevision` (`apps/game-server/src/durability/character_progression.rs`).
- In `0017`, `game_character_stance.last_stance_occurrence_id` is a UUIDv7 with no foreign key to
  the stance receipts, and the row is checked against a `stance_chain` view that a later receipt
  kind can join.

**UNKNOWN**, to be settled from Reference evidence by the lanes in §5:

- the Dawnport vocation-choice interaction and its departure rule;
- the magic-level formula and the per-vocation mana multipliers;
- the Global death loss of magic-level progress.

## 4. Decision

### 4.1 Storage

- **Table.** `game_character_build_state` holds one row per Character. Besides
  `committed_character_revision` and `last_build_occurrence_id`, which the row guard checks
  (§4.2), it has three fields:
  - `vocation`: `none` or a vocation key;
  - `magic_level`;
  - `mana_spent`: mana spent toward the next magic level. It resets on an advance (the
    Canary/Crystal `manaspent` semantics). This is not the cumulative total; W2b confirms it
    against Reference evidence.
- **Absence.** No row means `none`, 0 and 0, as in STANCE-0 §4.1. The revision-1 branch of the
  consistency guard rejects any build receipt and any build row (a row-only insert at revision 1
  fails there, because that branch returns before the row checks). The row is created by the
  first build receipt. It is never deleted, and truncate is rejected. Character creation inserts
  nothing, and existing Characters need no backfill.
  - Why not an initializer receipt: at revision 1 the chain admits no receipt (§3), and a receipt
    that advances no revision breaks the rule that receipts total `revision − 1`. An initializer
    that advanced the revision at admission would move the revision under the admission fence.
  - The first build receipt has `vocation_before = none`, `magic_level_before = 0` and
    `mana_spent_before = 0`.
- **Admission.** Admission loads the row, or the absent-row values, into the live Character, and
  `CasterState` reads from it. It writes nothing.

### 4.2 Receipt chain

- **Revisions and receipts.** Every change to build state is a CharacterRevision with exactly one
  receipt of a new kind, `game_character_build_receipts`, in the `0009` chain.
  - One revision carries one receipt of one kind (STANCE-0 §4.3, DEATH-0 §3.1-§3.2).
  - The receipt is immutable, and the chain cannot be truncated.
  - A build receipt needs an initialized progression row, as a stance receipt does.
- **Receipt shape.** CHAR-BUILD-1 defines the columns. Each receipt holds:
  - `build_occurrence_id` (UUIDv7, the key), `character_id`, `original_character_revision` and
    `committed_character_revision` = original + 1;
  - the before and after values of `vocation`, `magic_level` and `mana_spent`. At least one of
    them changes, so a no-op is not a receipt;
  - level and total experience before and after, which are equal, as in STANCE-0;
  - `stance_before` and `stance_after` (see "Stance fields");
  - a cause, with a CHECK on its direction, so one cause cannot pass as another (DEATH-0 §3.2):
    - `training`: vocation equal; (`magic_level`, `mana_spent`) strictly increases, compared in
      that order (an advance resets `mana_spent`);
    - `death_flush`: as `training`, committed immediately before a death (§4.6);
    - `vocation_choice`: `none` to a vocation key; `magic_level` and `mana_spent` equal;
    - `promotion`: a vocation key to another key; `magic_level` and `mana_spent` equal;
    - `death_loss`: vocation equal; (`magic_level`, `mana_spent`) strictly decreases, compared in
      that order (a lost magic level may leave more `mana_spent` toward the lower level);
  - the typed death reference, non-NULL exactly for `death_flush` and `death_loss`:
    `{family: death, key: death_occurrence_id, revision: death_committed_character_revision}`.
    The guard checks that this death receipt exists for the same Character, and that:
    - for `death_loss`, the death's committed revision equals this receipt's original revision;
    - for `death_flush`, the death's original revision equals this receipt's committed revision;
  - `command_binding` (1..1,024 B), `policy_digest` (32 B), the revision fields of an XP receipt and
    `committed_at`.
- **Stance fields.** Both are NULL unless the revision is a stance transition. A transition is
  allowed only for `vocation_choice` and `promotion`. It is a prune: `stance_before` is the stored
  non-NULL key and `stance_after` is NULL. So it is never the first stance transition and never
  inserts the stance row.
  - Only a receipt with differing stance values joins the STANCE-0 §4.3 stance chain. In `0017`
    this is a union into `stance_chain` of `build_occurrence_id AS occurrence_id WHERE
    stance_before IS DISTINCT FROM stance_after`.
  - When one does, the `game_character_stance` row takes its `stance_after`, its revision and its
    `build_occurrence_id` as `last_stance_occurrence_id`, in the same revision.
  - Any other build receipt leaves the stance row unchanged.
  - `last_stance_occurrence_id` may now name a build occurrence. The reference is typed by
    (`character_id`, `committed_character_revision`), which has exactly one receipt, and
    `stance_chain` carries its kind.
- **Occurrence key.** The owning session mints one UUIDv7 per commit attempt. It keeps the key
  until the outcome is known, and puts it in `command_binding`. The binding is a digest of the
  complete intent: character, expected revision, occurrence, cause, the before and after values,
  the stance fields, the death reference and the content and policy revisions.
- **Writer.** `commit_character_build` mirrors `commit_character_experience`:
  - It first takes the per-occurrence advisory lock `oteryn:character-build:<occurrence>` (as
    `oteryn:character-xp:<occurrence>`), then looks up a replay, then takes the same recovery
    fence, admission-relation locks, reconnect-session row, runtime-scope assignment,
    session-generation fence and `character_root` row lock.
  - The revision comes from `fence.expected_character_revision`.
  - XP, death, stance and build commits serialize on the `character_root` lock. The loser's fence
    is stale, so it writes nothing. For a training checkpoint, the runtime keeps the pending mana
    as a delta. It applies that delta to the latest committed after values (a `death_loss` may
    have changed them) and retries at the next revision with a new occurrence.
  - A retry with the same occurrence and binding returns the first receipt. The same occurrence
    with a different binding conflicts. The binding is compared before replay.
  - `reconcile_character_build` resolves an ambiguous outcome by occurrence.
- **State guard.** The equal-XP-and-level branch is shared with STANCE-0 and is told apart by the
  receipt kind.
- **Shared guard.** Build state joins the guard-rewrite chain of DEATH-0, STANCE-0 and H-1, in
  the #162 lease order (STANCE-0, then CHAR-BUILD-1, then H-1). Each rewrite starts from the last
  merged one and carries every receipt kind so far. The migration takes the next free number when
  it is allocated. The consistency guard counts build
  receipts in the `revision − 1` total and chains their level and experience.
- **Row guard.** `game_character_build_state` has a deferred constraint trigger on insert, update
  and delete, like the stance row.
  - A delete is rejected.
  - The row must equal the latest build receipt's after values, revision and occurrence. It is
    absent when no build receipt exists.
  - So a write that touches only the row fails at commit, and so does an XP, death or stance commit
    that changes it.

### 4.3 No vocation

- `Vocation` gains a `None` value.
- Before Dawnport, a spell is castable only if its Reference vocation list includes "none".
- A class spell stays rejected until the Character has chosen that class.

### 4.4 Vocation choice (D150)

- **The choice.** The Dawnport vocation choice is one revision that carries:
  - exactly one build receipt (cause `vocation_choice`). This is the vocation lane's own receipt
    from STANCE-0 §4.6;
  - `stance_before` and `stance_after` in that receipt, when the stored stance no longer fits. The
    receipt prunes it, and the STANCE-0 §4.3 stance chain counts it (§4.2 "Stance fields").

  There is no separate stance receipt, and the stance row is updated in the same revision. When no
  stance is pruned, both stance fields are NULL and the stance row is unchanged. The fit rule stays
  UNKNOWN (STANCE-0 §4.6).
- **Content.** The Dawnport island, the choice interaction and the departure rule come from
  Reference evidence. They belong to their own content lane.
- **Promotion.** Promotion (level 20, Premium, NPC) uses the same vocation writer in a later slice.

### 4.5 Magic-level training (D151)

- **Accumulation.** `mana_spent` accumulates in the live session with each cast's mana cost.
- **Commits.** It is committed as a build receipt:
  - on every magic-level advance, which is always durable. The live magic level changes only after
    that receipt commits;
  - before a death, as its own revision (§4.6);
  - at logout;
  - at a checkpoint of at most 60 seconds.

  A checkpoint is skipped when `mana_spent` has not changed since the last receipt, so receipts do
  not grow without progress.
- **Crash loss.** A crash may lose at most one checkpoint of mana-spent progress. It never loses a
  magic level.
- **Growth.** A training Character adds at most about one receipt a minute, and the chain cannot be
  truncated. `CharacterRevision` cannot be exhausted (NUMERIC(20,0)). Receipt growth per active
  Character is UNKNOWN; W2b measures it and reports before V1 ships. No resource row is added
  unless the measurement shows a need (as STANCE-0 §4.7).
- **Formula.** The advance formula and the per-vocation multipliers come from Reference evidence,
  cited in W2b. `vocation = none` uses its own Reference multiplier.

### 4.6 Death

- Death loss of magic-level progress follows Global. The loss amounts come from Reference evidence
  in the DEATH lane.
- The death writer's fence expects revision R. A death commits up to three consecutive revisions in
  one database transaction:
  1. R to R+1: the pending `mana_spent` flush (build receipt, cause `death_flush`, referencing the
     death), skipped when nothing is pending;
  2. R+1 to R+2: the unchanged DEATH-0 death receipt;
  3. R+2 to R+3: the magic-level loss (build receipt, cause `death_loss`, referencing the death),
     skipped when the loss changes nothing.

  Without a flush, each step moves down by one.
- Either every revision commits or none does. Each revision still carries exactly one receipt, so
  DEATH-0 needs no amendment.
- **One fence.** The composite holds the death writer's fence at R. The death receipt's
  `original_character_revision` is R+1 after a flush and R without one. This refines DEATH-0 §3.5:
  the death binding covers the death receipt's own original revision, not always the fence's
  expected revision. The two are equal until the DEATH ML loss child ships, so DEATH-1 replay is
  unchanged.
- **Retry.** The composite first takes the death occurrence's advisory lock and looks up the death
  occurrence.
  - The binding is compared before replay (DEATH-0 §3.5). The runtime keeps its pending mana until
    the outcome is known, so a retry recomputes the same binding, with R+1 exactly when a flush was
    pending.
  - If the death receipt exists and the binding matches, it returns the committed revisions and
    writes nothing. It finds the flush and the loss through their typed death reference.
  - A different binding conflicts.
  - An attempt that did not commit left nothing, so a retry mints new build occurrences.
- **Reconcile.** `reconcile_character_death` reports every revision by the same lookup.
- **Before CHAR-BUILD-1.** DEATH-1 commits only the death receipt. The flush and the loss are added
  by the DEATH ML loss child.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| CHAR-BUILD-1 | Migration, writer and reconcile: the table, the receipt kind, admission load, row and chain guards (§4.1-§4.2). It needs a persistence review. | this decision; the guard chain order |
| W2b | `CasterState` facts from build state, `Vocation::None`, training accumulation and the Reference formula. Allocated only after CHAR-BUILD-1, which tightens ruling 5896480875. | CHAR-BUILD-1 |
| DAWNPORT-1 | Dawnport content and the vocation-choice interaction | CHAR-BUILD-1 |
| DEATH ML loss | Magic-level progress loss at death and the composite of §4.6 | CHAR-BUILD-1, DEATH-1 |

## 6. Rejected options

- **Choice at creation (control-plane option a).** The owner chose Global parity (D150).
- **A dev-only starter profile (option c).** It is not durable and would be replaced anyway.
- **Columns on `game_character_progression_state`.** Build state changes on its own events. A
  separate row and receipt kind keep the XP receipts unchanged, following the A11 pattern.
- **An initializer receipt at creation or first admission.** The chain admits no receipt at
  revision 1, and advancing the revision at admission would move it under the admission fence
  (§4.1).
- **Equal non-NULL stance fields on every build receipt.** The guard would have to check them
  against the stance chain on every build commit, for no information.
- **A receipt per cast.** It adds a durable write to every cast, which costs too much for
  mana-spent progress. The checkpoint bound (§4.5) caps the loss instead.

## 7. Decision test

- **Must decide now:** YES. Every production cast is rejected until `CasterState` has facts.
- **Minimum sufficient:** one row, one receipt kind and the Global acquisition path.
- **Superseding evidence:** a Reference rule that contradicts §4.3-§4.6.
- **Deliberately not decided:** promotion details, skills other than magic level, and
  vitals persistence.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5896414182 (SPELL-CASTER-FACTS) and 5896342127"
owner_decisions: [D150, D151]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # CHAR-BUILD-1 may be allocated; W2b and DAWNPORT-1 follow it
required_fresh_allocation: true
required_independent_review: "CHAR-BUILD-1 persistence review (receipt kind, guards, fences)"
implementation_lanes: [CHAR-BUILD-1, W2b, DAWNPORT-1, DEATH ML loss]
remaining_unknowns:
  - Dawnport choice and departure details
  - magic-level formula and multipliers
  - death loss amounts
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates CHAR-BUILD-1."
```

## 9. Before-freeze checklist

1. Contract amendments: the spell cast contract §10 points here. STANCE-0 and DEATH-0 need no text
   amendment: the vocation receipt is STANCE-0 §4.6's combined receipt, and each revision carries
   one receipt. The death binding's original revision (§4.6 "One fence") is a refinement for the
   DEATH ML loss child and changes nothing before it ships.
2. Serialization: `character_root` row lock and the session-generation fence. The losing writer
   writes nothing (§4.2 "Writer").
3. Restart: the row and the receipts hold every committed fact. At most one checkpoint of
   `mana_spent` is lost (§4.5). A death composite is recovered by its death occurrence (§4.6).
4. Typed references: the death reference is `{family, key, revision}`, and
   `last_stance_occurrence_id` is typed by (`character_id`, `committed_character_revision`) and
   the `stance_chain` kind (§4.2).
5. Wire: no wire or schema change for clients in this decision.
6. Split work: the death composite is one transaction under one fence, retried and reconciled by
   the death occurrence, with the flush and the loss found through their typed references (§4.6).
7. Self-review: `oteryn-hard-worker`, read-only, on the complete draft. Its material finding (the
   flush found by revision could be an earlier checkpoint) is fixed by the `death_flush` cause and
   reference; its evidence gap (no per-cause direction) by the cause CHECKs.
