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
  against `0016`, the STANCE-0 migration `0017` and the before-freeze checklist (§9), and again
  after the independent review of `ef18a7ca` (#1271 5898945224)
- Supersedes these clauses of ruling 5896480875, under escalation A13-RECEIPT-CHAIN (#162
  5897202372):
  - storage, "a missing row is invalid after creation; creation inserts none/0/0": replaced by
    §4.1 "Absence";
  - vocation_choice, "plus the A11 stance-prune receipt": replaced by §4.4, one combined receipt;
  - death, "a build receipt in the same revision as the death receipt": replaced by §4.6, where the
    death receipt carries the loss (DEATH-0 §3.1 amendment);
  - `implementation_may_resume` for W2b: W2b now follows CHAR-BUILD-1 (§5).
- Amends: DEATH-0 §3.1 (death receipt build fields, §4.6), in the DEATH-0 document in this PR
- Decides: the "Magic-level training" item in
  `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §10
- Migration, runtime, content and production authority: **NONE**. Each lane in §5 changes code
  under its own allocation and review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **CHAR-BUILD-1** (durability lane, `oteryn-hard-worker`, persistence review). Owned paths:
  the next free migration at allocation (after `0018`-`0020`), `apps/game-server/src/durability/**`
  for the writer and reconcile, the admission load, and its `*_postgres` tests. It builds:
  - `game_character_build_state` and `game_character_build_receipts` (§4.1-§4.2). No row means
    (`none`, 0, 0). No creation insert, no backfill.
  - The build kind in the shared `0009` guards, in that migration. The #162 lease
    orders the guard rewrites STANCE-0 (`0017`, merged `878f6fad`), then CHAR-BUILD-1, then H-1
    (spell contract §8.2). CHAR-BUILD-1 starts from the last merged rewrite (`0018` of DEATH-1a,
    #1278, if merged) and carries XP, death, stance and build (§4.2 "Shared guard").
  - The extension of `verify_character_integrity` and the grants (§4.2 "Integrity verifier",
    "Grants").
  - `commit_character_build` and `reconcile_character_build` (§4.2 "Writer").
  - The nullable build columns on `game_character_death_receipts` (DEATH-0 §3.1 amendment, §4.6).
  - The admission load into the live Character (§4.1).
  - Tests: a row-only write fails, also at revision 1; a revision-1 Character has no build
    receipt; each cause rejects the wrong direction; the chain holds
    across XP, death, stance and build; the build chain rejects a receipt whose before values
    differ from the previous build-carrying after values (§4.2 "Build chain"); the stance chain
    counts only a build receipt whose stance fields differ (§4.2 "Stance fields"); replay,
    conflict and a stale fence; one red run per guard branch.
- **W2b** (spell lane, after CHAR-BUILD-1). `CasterState` reads build state, adds
  `Vocation::None` (§4.3), accumulates training and commits checkpoints through
  `commit_character_build` (§4.5). The formula is cited from Reference evidence. It measures
  receipt growth and guard latency against chain length (§4.5 "Growth").
- **DAWNPORT-1** (content lane). The choice interaction calls the vocation writer with the combined
  stance fields (§4.4).
- **DEATH ML loss** (DEATH lane, after CHAR-BUILD-1 and DEATH-1). The flush before a death and the
  death receipt's build fields (§4.6).
- Binding sections: §4.1-§4.6 of this document; STANCE-0 §4.3 and §4.6; DEATH-0 §3.1 (as amended),
  §3.2 and §3.5; DUR-02 rule 2 (one CharacterRevision per semantic transaction).

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
  `0017`, merged in #1270 as `878f6fad`, "initial Character progression is inconsistent"). For
  revision > 1 the receipts of all kinds total `revision − 1`.
- The D88 progression initializer inserts the progression row without a receipt and without
  advancing `CharacterRevision` (`apps/game-server/src/durability/character_progression.rs`).
- DUR-02 rule 2 (`DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` §5): every
  Character semantic transaction advances `CharacterRevision` exactly once. It may change several
  typed child relations in that one revision.
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
    - `vocation_choice`: `none` to a vocation key; `magic_level` and `mana_spent` equal;
    - `promotion`: a vocation key to another key; `magic_level` and `mana_spent` equal;
  - There is no death cause: the loss is carried by the death receipt itself (§4.6);
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
  the stance fields and the content and policy revisions.
- **Writer.** `commit_character_build` mirrors `commit_character_experience`:
  - It first takes the per-occurrence advisory lock `oteryn:character-build:<occurrence>` (as
    `oteryn:character-xp:<occurrence>`), then looks up a replay, then takes the same recovery
    fence, admission-relation locks, reconnect-session row, runtime-scope assignment,
    session-generation fence and `character_root` row lock.
  - The revision comes from `fence.expected_character_revision`.
  - XP, death, stance and build commits serialize on the `character_root` lock. The loser's fence
    is stale, so it writes nothing. For a training checkpoint, the runtime keeps the pending mana
    as a delta. It applies that delta to the latest committed after values (another commit may have
    changed them) and retries at the next revision with a new occurrence.
  - The owning session sends one Character's commits (XP, stance, build, death) through one
    ordered queue, so its own checkpoint never makes its own next commit stale. Each commit takes
    the fence at the revision the previous one committed.
  - A retry with the same occurrence and binding returns the first receipt. The same occurrence
    with a different binding conflicts. The binding is compared before replay.
  - `reconcile_character_build` resolves an ambiguous outcome by occurrence.
- **State guard.** The equal-XP-and-level branch is shared with STANCE-0 and is told apart by the
  receipt kind.
- **Shared guard.** Build state joins the guard-rewrite chain of DEATH-0, STANCE-0 and H-1, in
  the #162 lease order (STANCE-0, then CHAR-BUILD-1, then H-1). Each rewrite starts from the last
  merged one and carries every receipt kind so far. The migration takes the next free number at
  allocation, after `0018` (DEATH-1a, #1278) and the `0019`/`0020` CHARM reservations.
  - The consistency guard counts build receipts in the `revision − 1` total, and chains their
    level and experience.
  - It keeps every arm it has when CHAR-BUILD-1 starts: the count and distinct count, the root
    state match, the cross-kind before/after chain, the revision fields, the stance chain, and the
    rule that the successor explains the state change.
- **Integrity verifier.** `verify_character_integrity`
  (`apps/game-server/src/durability/character_authority.rs`) checks the chain at admission.
  CHAR-BUILD-1 extends it from the latest merged version (XP ∪ death ∪ stance after #1278) to
  build receipts, and to death receipts' build fields. It keeps every existing arm. Otherwise
  every Character with a build receipt would fail admission as `Unavailable`.
- **Grants.** As in `0017`:
  - REVOKE ALL on the new tables and guard functions from PUBLIC;
  - `oteryn_game_runtime` gets SELECT and INSERT on the build receipts, and SELECT, INSERT and
    UPDATE on the build row (never DELETE);
  - `oteryn_game_control` gets SELECT on both;
  - the guard functions get a fixed `search_path`.

  The new death receipt columns need no new grant.
- **Build chain.** The shared consistency guard reads a `build_chain`: build receipts, and death
  receipts with non-NULL build fields (§4.6), ordered by revision. Each receipt's before values
  equal the previous one's after values, or (`none`, 0, 0) for the first. A death's single
  `vocation` counts as both its before and after. This mirrors `stance_chain` in `0017`.
- **Row guard.** The row check runs in the shared consistency guard. `game_character_build_state`
  has a deferred constraint trigger on insert, update and delete, like the stance row, and the
  build receipts get one on insert (death receipts already have the `0016` trigger). So a receipt
  committed without its row update also fails. The revision-1 branch lists build receipts and the
  build row among the relations that must not exist.
  - A delete is rejected.
  - The row must equal the after values, revision and occurrence of the latest receipt that
    carries build state: a build receipt, or a death receipt with non-NULL build fields (§4.6). It
    is absent when no such receipt exists.
  - So a write that touches only the row fails at commit, and so does an XP or stance commit, or a
    death commit without build fields, that changes it.

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
  - before a death, in its own earlier transaction (§4.6);
  - at logout;
  - at a checkpoint of at most 60 seconds.

  A checkpoint is skipped when `mana_spent` has not changed since the last receipt, so receipts do
  not grow without progress.
- **Crash loss.** A crash may lose at most one checkpoint of mana-spent progress. It never loses a
  magic level.
- **Growth.** A training Character adds at most about one receipt a minute, and the chain cannot be
  truncated. `CharacterRevision` cannot be exhausted (NUMERIC(20,0)). Receipt growth per active
  Character is UNKNOWN. The `0017` consistency guard reads the whole chain on every row event, so
  W2b measures both receipt count and guard latency against chain length, and reports before V1
  ships. No resource row is added unless the measurement shows a need (as STANCE-0 §4.7).
- **Formula.** The advance formula and the per-vocation multipliers come from Reference evidence,
  cited in W2b. `vocation = none` uses its own Reference multiplier.

### 4.6 Death

- Death loss of magic-level progress follows Global. The loss amounts come from Reference evidence
  in the DEATH lane.
- **One revision per death.** A death stays one semantic transaction that advances
  `CharacterRevision` once, with exactly one receipt: the DEATH-0 death receipt (DUR-02 rule 2,
  DEATH-0 §3.1). No build receipt is written in the death transaction.
- **Flush first.** Pending `mana_spent` is committed before the death as an ordinary `training`
  checkpoint, in its own earlier transaction through `commit_character_build` (§4.5). It is skipped
  when nothing is pending. The death then takes its fence at the revision the flush committed, so
  DEATH-0 §3.5 (fence, binding and replay) is unchanged.
  - The death binding is built only after the flush commits, because it covers the expected
    revision.
  - An ambiguous flush goes through `reconcile_character_build` before the death takes its fence.
    A failed or stale flush retries as in §4.2 "Writer".
  - The death commits only after the flush has committed or nothing is pending, so no mana escapes
    the loss by being applied on top of the lowered values.
  - If the process fails between the two, the flush stays committed. An uncommitted death is lost
    like any crash before commit, and the Character is admitted at the flush revision.
- **Loss in the death receipt (DEATH-0 §3.1 amendment).** The death receipt gains nullable build
  fields: `vocation`, and the before and after values of `magic_level` and `mana_spent`.
  - CHECKs: all NULL or all non-NULL; `vocation` is `none` or a vocation key; `magic_level` and
    `mana_spent` are non-negative and bounded as in the build row; the direction below as a row
    comparison.
  - All are NULL, or all are non-NULL. NULL means the death did not change build state (nothing
    to lose, or a death committed before the DEATH ML loss child ships).
  - When non-NULL: before equals the latest build-carrying receipt's after values (or `none`, 0, 0
    when there is none); vocation is unchanged; (`magic_level`, `mana_spent`) strictly decreases,
    compared in that order (a lost magic level may leave more `mana_spent` toward the lower
    level).
  - A death is never the first build-carrying receipt, because (0, 0) cannot strictly decrease. So a
    death only updates the build row, never inserts it. The row takes the after values in the same
    revision, with the death occurrence as its last occurrence. The row guard (§4.2) counts this
    receipt.
  - Binding: with NULL fields, the death binding keeps the DEATH-1 version 1 bytes unchanged. With
    non-NULL fields, it uses a new binding version that adds the build fields, so a retry across
    the deploy of the DEATH ML loss child still replays.
- **Retry and reconcile.** Unchanged from DEATH-0 §3.5: by death occurrence, binding compared
  before replay. The flush is an ordinary build receipt with its own occurrence.
- **Before the DEATH ML loss child.** DEATH-1 writes NULL build fields and needs no change. Until
  that child merges, a death does not lower magic-level progress, which is not Global parity
  (D151). So W2b may merge, but training is not enabled in production until the DEATH ML loss
  child has merged. Dev and test builds may enable it earlier.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| CHAR-BUILD-1 | Next free migration at allocation, writer and reconcile: the table, the receipt kind, the nullable death receipt build columns, admission load, row and chain guards, integrity verifier, grants (§4.1, §4.2, §4.6). It needs a persistence review. | this decision; `0018`-`0020` |
| W2b | `CasterState` facts from build state, `Vocation::None`, training accumulation and the Reference formula. Allocated only after CHAR-BUILD-1, which tightens ruling 5896480875. | CHAR-BUILD-1 |
| DAWNPORT-1 | Dawnport content and the vocation-choice interaction | CHAR-BUILD-1 |
| DEATH ML loss | The flush before a death and the death receipt's build fields (§4.6) | CHAR-BUILD-1, DEATH-1 |

## 6. Rejected options

- **Choice at creation (control-plane option a).** The owner chose Global parity (D150).
- **A dev-only starter profile (option c).** It is not durable and would be replaced anyway.
- **Columns on `game_character_progression_state`.** Build state changes on its own events. A
  separate row and receipt kind keep the XP receipts unchanged, following the A11 pattern.
- **An initializer receipt at creation or first admission.** The chain admits no receipt at
  revision 1, and advancing the revision at admission would move it under the admission fence
  (§4.1).
- **Several revisions in one death transaction (flush, death, loss).** DUR-02 rule 2 allows one
  CharacterRevision per semantic transaction; amending an owner baseline is not needed for this.
- **A separate later loss transaction.** A failure between the death and the loss would lose the
  penalty or need a new obligation record; the death receipt's build fields avoid both.
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

1. Contract amendments: the spell cast contract §10 points here. DEATH-0 §3.1 is amended in its own
   document in this PR (death receipt build fields). STANCE-0 needs none: the vocation receipt is
   its §4.6 combined receipt. DUR-02 rule 2 holds: every transaction advances one revision.
2. Serialization: `character_root` row lock and the session-generation fence; the losing writer
   writes nothing; one session orders its own commits (§4.2 "Writer").
3. Restart: the row and the receipts hold every committed fact. At most one checkpoint of
   `mana_spent` is lost (§4.5). A flush committed before a failed death stays valid (§4.6).
4. Typed references: `last_stance_occurrence_id` and the build row's last occurrence are typed by
   (`character_id`, `committed_character_revision`), which has exactly one receipt, and the guard
   reads its kind (§4.2).
5. Wire: no wire or schema change for clients in this decision.
6. Split work: the flush and the death are separate transactions, each complete on its own; the
   loss is atomic with the death in one receipt (§4.6).
7. Self-review: `oteryn-hard-worker`, read-only, on the complete draft of `ef18a7ca`, and again on
   this repair.
