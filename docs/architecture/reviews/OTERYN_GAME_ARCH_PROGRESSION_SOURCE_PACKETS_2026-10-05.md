# ARCH-PROGRESSION-SOURCE-0: the Character progression content source and its composition

- Decision id: ARCH-PROGRESSION-SOURCE-0 (CP D699).
- Status: the §1 rulings and the §2 packets are accepted on merge, except the three items of
  §1.6, which are flagged for owner acceptance. Until the owner accepts them, PROGRESSION-CONTENT-1
  may be authored but not frozen, and PROGRESSION-OWNER-1 is not allocated.
- Origin:
  - ARCH-KILL-REWARD-LOGOUT-1 (#1802) §0.3 and §1.5: live XP and Bestiary wait on a separate
    PROGRESSION-OWNER-1 packet. Its Codex P2 4180039074 is deferred into KILL-REWARD-COMP-1.
  - The PROGRESSION-OWNER-1 worker report (CP, after #1802): the input that packet assumed does
    not exist on `main` (§0.1).
  - Reference evidence: GAME-CHAR-01 Stage B delta 02 E9 (the tibia.com experience table is the
    primary evidence), `docs/reference/tibia-manual/characters.md` §5.1.1 and §5.1.11.
  - D58 and D68: death loss ratio 1/1 with `Floor` rounding; the loss formula is code
    (`domain/death.rs`).

## 0. Gaps and order

### 0.1 What is missing on `main` (8f174761c)

- **No finite progression policy is built outside tests.** `FiniteProgressionPolicy`
  (`domain/progression.rs`) is constructed only in unit tests and PG fixtures. No content file,
  decoder or producer yields a threshold table.
- **The rulesets are empty.** `rulesets/character/experience/` and `rulesets/character/death/`
  hold only their `index.json` with `population_state: READY_UNPOPULATED`.
- **Nothing pins progression revisions.** `AdmittedSession` (`gameplay_transport/connection.rs`),
  the readiness and `ArtifactExpectation` (`content/artifact.rs`) and the native gameplay pin
  (`content/native_gameplay.rs`) carry no progression revision.
- **The durable row holds revisions only.** `game_character_progression_state` stores the eight
  revisions (profile, ruleset, content, simulation, evidence, declaration, policy, reward) and the
  level and total; receipts store `policy_digest`. The thresholds themselves are never stored, so
  the policy must come from pinned content at runtime.
- **The accessor is a placeholder.** `player_death_progression()`
  (`gameplay_transport/mod.rs`) is a `const fn` returning `None` with `N = 1`, which
  `validate_policy` could never accept (`N >= 2`). Its only caller is `respawn_after_death`.
- **Ownership.** `gameplay_transport/mod.rs` and `connection.rs` are owned by ATTACK-1b #1798,
  which is still open.

### 0.2 Exact durable constraints this decision must respect

- `stored_context_matches` (`durability/character_progression.rs`) requires all eight stored
  revisions to equal the request exactly. Any other value is `ProgressionContextMismatch`: the
  award or death write is refused. There is no progression migration path.
- The request's profile, ruleset and content revisions must equal the Character root, and the
  root must equal its latest `game_character_interpretations` row.
- `initialize_character_progression` admits the row only while the Character root is at
  revision 1. A missing row after revision 1 is `InvalidStoredState`.
- Every revision must pass `valid_revision`: at most 128 bytes, ASCII alphanumeric first,
  then alphanumeric or `._:-`.

### 0.3 Order

1. This decision merges, and the owner accepts §1.6.
2. PROGRESSION-CONTENT-1 (§2.1) populates the rulesets, the reference snapshot, the producer and
   the pinned `progression` section with its decoder. It does not touch `gameplay_transport/`
   and does not wait for #1798.
3. PROGRESSION-OWNER-1 (§2.2) runs after ATTACK-1b #1798 and PROGRESSION-CONTENT-1 merge. It
   composes the binding into `AdmittedSession` and replaces `player_death_progression()`.
4. KILL-REWARD-COMP-1 is not reordered. It lands loot and the corpse with
   `no_progression_binding` (ARCH-KILL-REWARD-LOGOUT-1 §1.5). Whichever of KILL-REWARD-COMP-1
   and PROGRESSION-OWNER-1 merges second switches the kill XP and Bestiary arm on (§2.2).

## 1. Rulings

### 1.1 The content source

- **Experience table.** `rulesets/character/experience/experience-table.json`, schema
  `OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1`:
  - `levels`: exactly `CHARACTER_EXPERIENCE_TABLE_LEVELS` rows `{level, minimum_experience}`,
    levels `1..=2000` in order, with `minimum_experience` as a decimal string;
  - `terminal_exclusive_experience`: the minimum experience of level 2001;
  - `revision` and `evidence_revision` (§1.3).
- **Derivation.** The values are Reference-derived. A producer
  (`tools/content-schema/character-progression/`) computes each level from the Reference
  formula `50/3 * (L^3 - 6L^2 + 17L - 12)`, so level 1 is 0 and level 2 is 100, and checks it
  against a checked-in snapshot of the tibia.com experience table
  (`docs/reference/experience-table-20261005/`). Every level that the snapshot lists must match
  exactly, or the producer fails. Levels above the highest level the snapshot lists are
  formula-derived and are named in the declared differences.
- **Fixed length.** `CHARACTER_EXPERIENCE_TABLE_LEVELS = 2000` is one crate constant, used as `N`
  by the decoder, the death path and the kill reward path. A file of any other length is
  refused at decode. Level 2000 needs about 1.33e11 experience, far inside `i64`.
- **Death policy.** `rulesets/character/death/death-policy.json`, schema
  `OTERYN_GAME_CHARACTER_DEATH_POLICY/v1`: `revision`, `loss_numerator: 1`,
  `loss_denominator: 1`, `rounding: "floor"`. Any other ratio or rounding is refused at decode,
  as `validate_policy` already does (D58, D68). The loss formula, blessing and promotion
  reductions stay in `domain/death.rs`. The file pins the revision and does not reopen the
  formula.
- **Reward policy.** `rulesets/character/experience/reward-policy.json`, schema
  `OTERYN_GAME_CHARACTER_REWARD_POLICY/v1`: `revision` only, naming the current kill reward
  rule. The base experience is the creature's `profile.experience`, there is one principal
  (`COMBAT01_REWARD_PRINCIPALS_MAX`), and no stamina, multiplier, party or premium rule
  applies. A later rule is a new file revision.
- **Declared differences.** `rulesets/character/experience/declared-differences.json`, schema
  `OTERYN_GAME_CHARACTER_PROGRESSION_DIFFERENCES/v1`: `revision` and a list of
  `{id, reference, oteryn}` records. Version 1 lists at least:
  - the qualitative low-level experience bonus below level 50 (`characters.md` §5.1.1, no
    formula in evidence): not modelled;
  - any formula-derived levels above the snapshot's coverage;
  - stamina and experience boosts: not modelled.
- Both `index.json` files move to `POPULATED`, and their `notes` name the new files, like
  `rulesets/progression/bestiary/`.

### 1.2 How a World pins the content

- The native gameplay manifest gains one optional pinned section, `progression`, schema
  `OTERYN_NATIVE_PROGRESSION/v1`. It is one document that holds the decoded table, the death
  policy, the reward policy revision, the declared differences revision and the revisions of
  §1.3, copied by the native gameplay manifest producer from the four ruleset files. Its digest
  is bound into the native gameplay pin and the outer artifact digest like every other section.
- **Strict decode.** The section is decoded once into an immutable `CharacterProgressionContent`
  (new `content/character_progression_content.rs`), which runs the same checks as
  `validate_policy` on a template context. A malformed section refuses the manifest. The
  runtime never reads the `rulesets/` files.
- **Absent section.** An existing pin without the section stays valid and decodes to no
  progression content. The World then has no progression binding: player death stays on the
  current non-durable respawn path, and kill XP and Bestiary log `no_progression_binding`.
- **One section per World.** A World pins exactly one progression content. A Channel never
  mixes two.

### 1.3 Revisions, and which ones admission pins

| Revision | Source | Value |
| --- | --- | --- |
| `policy_revision` | World pin | `character-progression-policy-v1-<sha256-32>` of the canonical pair `{experience_table, death_policy}` of the two file revisions below |
| experience table revision (not a policy field) | World pin | `character-experience-v1-<sha256-32>` of the canonical table file |
| `death_policy_revision` | World pin | `character-death-v1-<sha256-32>` of the canonical death policy file |
| `reward_revision` | World pin | `character-reward-v1-<sha256-32>` of the canonical reward policy file |
| `declaration` = `declared_difference_revision` | World pin | `character-progression-differences-v1-<sha256-32>` |
| `evidence` | World pin | `experience-table-evidence-20261005-<sha256-32>` of the reference snapshot |
| `simulation` | World pin | `oteryn-simulation-determinism-exact-i64-v1`, the numeric profile of the projection |
| `profile`, `ruleset`, `content` | Character root | the root's revisions, which equal its latest interpretation |

- `<sha256-32>` is the first 32 lowercase hexadecimal digits of the SHA-256 of a digest input
  that never contains the revision being computed:
  - for a ruleset file, the canonical JSON bytes (sorted keys, no insignificant whitespace) of
    the document with its own top-level `revision` member removed. Every other member, such as
    the table's `evidence_revision`, stays in the input;
  - for `policy_revision`, the canonical JSON bytes of
    `{"death_policy": <death_policy_revision>, "experience_table": <experience table revision>}`;
  - for `evidence`, the raw bytes of exactly one file,
    `docs/reference/experience-table-20261005/levels.csv`. It is the normalized extract of the
    tibia.com experience table: UTF-8, LF line ends, a final LF, no header, one
    `<level>,<experience>` line per listed level in ascending order, decimal digits only. It
    holds no revision. The directory's `README.md` records the source URL, the capture date and
    the extraction method, and is not part of the digest.
- The producer computes each revision and writes it into the file's `revision` member. The
  decoder removes that member, recomputes the digest and refuses a file whose stated revision
  disagrees. A content change therefore always changes the revision, and an unchanged file
  keeps it.
- **Death policy durability.** `game_character_progression_state` stores no
  `death_policy_revision`. The death policy is therefore folded into the stored
  `policy_revision`: a change of `death-policy.json` alone changes `policy_revision`, so an
  existing Character's row no longer matches and its XP and death writes are refused (§1.6 B).
  No migration is needed.
- `simulation` is the progression section's own value, not the World's `sim_profile_revision`.
  An unrelated simulation profile change must not refuse every Character's progression writes.
- **Admission pins** all nine values for the life of the session, and the row stores eight
  of them; the ninth, `death_policy_revision`, is bound through `policy_revision`:
  - the six World values come from the generation that admits the session, through its
    `CharacterProgressionContent`;
  - the three Character values are read from the Character root at admission, as the durable
    layer already requires. Admission adds no new compatibility check between them and the
    World.
- A session keeps its pinned binding until it ends. A generation change that alters the
  progression section reaches a Character at its next admission.

### 1.4 Composition

- At generation activation, the active generation exposes its `CharacterProgressionContent`
  (one shared immutable value, behind `Arc`).
- At play admission, the session builds one
  `RewardProgressionBinding<CHARACTER_EXPERIENCE_TABLE_LEVELS>` from that content and the
  Character root's three revisions, and stores it in `AdmittedSession`. The
  `BestiaryProgressionBinding` is derived from it.
- `player_death_progression()` is removed. `respawn_after_death` reads the binding of the dead
  actor's own session. KILL-REWARD-COMP-1 reads the binding of the principal's session, which
  already drains the settlement queue.
- Each award still clones the 2000-row table into its request (32 KiB). This cost is accepted. A
  measured cost would justify a borrowed request type.

### 1.5 Initialization

- At play admission, when the World pins progression content, admission reads the Character's
  progression row (`read_character_progression`) before the session enters the world:
  - **no row, root at revision 1:** admission calls `initialize_character_progression` with the
    binding's request. A retryable failure refuses the admission as retryable. On success the
    session keeps the binding;
  - **no row, root past revision 1:** admission does not call the initializer, which would
    return `InvalidStoredState`. The session is admitted with **no binding** and logs
    `progression_unbound reason=progression_uninitialized` once. There is no backfill;
  - **a row whose eight stored revisions differ from the binding:** the session is admitted
    with **no binding** and logs `progression_unbound reason=progression_context_mismatch` once
    (§1.6, item B);
  - **a row that matches:** the session keeps the binding.
- A session with no binding behaves exactly like a World without the section (§1.2): a player
  death takes the current non-durable respawn path and terminates, and kill XP and Bestiary log
  `no_progression_binding`. No durable progression write is attempted, so no death or award is
  left retrying against a row that can never accept it.
- A durable death write that fails retryably for a bound session keeps the existing
  `respawn_after_death` retry. Only the two unbound cases above can never succeed, and they are
  decided at admission.
- Level-dependent runtime values are not in scope. `PLAYER_LEVEL_UNTIL_PROGRESSION_OWNER`
  (movement speed) stays until a later packet reads the stored level.

### 1.6 Flagged for owner acceptance

- **A. Pin-schema change.** §1.2 adds the `progression` section (`OTERYN_NATIVE_PROGRESSION/v1`)
  to the native gameplay pin format. It can later only be added to or superseded by a new
  schema.
- **B. Revision irreversibility.** Once a Character is initialized, its row holds the eight
  stored values of §1.3, with the death policy bound through `policy_revision`. Any later change to the table, death policy, reward policy, declared
  differences, evidence snapshot or simulation value changes a revision. Every existing
  Character is then refused XP and death writes until a progression migration owner exists.
  This decision builds no such owner, so a content change requires one first.
- **C. No backfill.** A Character created before PROGRESSION-OWNER-1 and already past root
  revision 1 never gains progression (§1.5). This affects test and qualification Characters
  only, since there is no production. Re-creating them is the remedy until a backfill decision.

## 2. Packets

### 2.1 PROGRESSION-CONTENT-1 (the progression content and its pin)

```yaml
task_id: OTV2-20261005-progression-content-1
decision: ARCH-PROGRESSION-SOURCE-0 §1.1-§1.3
depends_on: [ARCH-PROGRESSION-SOURCE-0 owner acceptance of §1.6 A and B]
worker: oteryn-impl-worker
review: content (Codex), on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - rulesets/character/experience/index.json
  - rulesets/character/experience/experience-table.json       # new
  - rulesets/character/experience/reward-policy.json          # new
  - rulesets/character/experience/declared-differences.json   # new
  - rulesets/character/death/index.json
  - rulesets/character/death/death-policy.json                # new
  - docs/reference/experience-table-20261005/**                # new: the tibia.com snapshot and its README
  - tools/content-schema/character-progression/**              # new: the producer and its tests
  - tools/content-schema/native-gameplay/**                    # the progression section only, and its tests
  - tools/qualification/node_boot/**                           # the progression staging only
  - apps/game-server/src/content/character_progression_content.rs        # new: decode, CHARACTER_EXPERIENCE_TABLE_LEVELS
  - apps/game-server/src/content/character_progression_content_tests.rs  # new
  - apps/game-server/src/content/native_gameplay.rs            # the optional progression section and its pin only
  - apps/game-server/src/content/mod.rs                        # the module line only
  - docs/agents/tasks/archive/OTV2-20261005-progression-content-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python3 -m unittest discover -s tools/content-schema/character-progression -p 'test_*.py'
  - python3 -m unittest discover -s tools/content-schema/native-gameplay -p 'test_*.py'
  - python3 tools/content-schema/validate_materialized_game_tree.py
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:** the four ruleset files and the two `index.json` moves (§1.1), the reference
  snapshot, the producer with its formula and snapshot check and its revision computation (§1.3),
  the `progression` pinned section in the native gameplay producer and decoder (§1.2), and
  `CharacterProgressionContent` with the crate constant.
- **Shared file.** `native_gameplay.rs` and `tools/content-schema/native-gameplay/**` are also
  owned by KILL-REWARD-COMP-1 for its `loot_tables` section. The two sections are disjoint; the
  second packet to merge merges `main` first.
- **Acceptance:**
  - A producer test that level 1 is 0, level 2 is 100, and every level the snapshot lists
    matches the formula; a mutated snapshot row fails the producer.
  - A producer test that a change of canonical content outside the `revision` member (one
    threshold value, the death `rounding`, one declared difference record, one
    `levels.csv` digit) changes the affected revision, that a whitespace or key-order change
    alone does not, that rewriting only the `revision` member does not change the
    recomputed digest, that a change of `death-policy.json` alone changes `policy_revision`, and
    that every revision passes `valid_revision`. A `levels.csv` with CRLF line ends, a header or
    an unsorted line is refused.
  - Decode tests (`include_str!` of the ruleset files, like `domain/bestiary.rs`): the checked-in
    files decode; 1999 or 2001 rows, a non-increasing threshold, a terminal at or below level
    2000, a death ratio other than 1/1 or a rounding other than floor, and a stated revision that
    disagrees with the recomputed one are each refused.
  - A native gameplay test that a pin with the section decodes to the same
    `CharacterProgressionContent`, a pin without it decodes to none, and a tampered section fails
    the digest.
  - No `gameplay_transport/` or `durability/` file changes.

### 2.2 PROGRESSION-OWNER-1 (re-issued: compose the progression binding)

```yaml
task_id: OTV2-20261005-progression-owner-1
decision: ARCH-PROGRESSION-SOURCE-0 §1.3-§1.5; ARCH-KILL-REWARD-LOGOUT-1 §1.5
depends_on: [ATTACK-1b #1798, PROGRESSION-CONTENT-1]
worker: oteryn-hard-worker
review: persistence (Codex), on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - apps/game-server/src/content/activation.rs                  # the ActiveGeneration progression accessor only
  - apps/game-server/src/combat/death_reward.rs                 # RewardProgressionBinding::from_content only
  - apps/game-server/src/gameplay_transport/character_progression_binding.rs        # new: per-session binding, initialization at admission
  - apps/game-server/src/gameplay_transport/character_progression_binding_tests.rs  # new
  - apps/game-server/src/gameplay_transport/connection.rs       # the AdmittedSession binding field and its three construction sites only
  - apps/game-server/src/gameplay_transport/mod.rs              # module line, removal of player_death_progression, respawn_after_death reading the session binding, and the kill reward accessor if KILL-REWARD-COMP-1 merged first
  - apps/game-server/src/gameplay_transport/kill_reward.rs      # the binding read only, if KILL-REWARD-COMP-1 merged first
  - apps/game-server/tests/support/character_progression_admission_postgres_cases.rs  # new
  - apps/game-server/tests/character_progression_postgres.rs    # registration only
  - docs/agents/tasks/archive/OTV2-20261005-progression-owner-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - the character_progression_postgres suite against PostgreSQL (repository CI service)
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:** the generation accessor (§1.4), the per-session binding and the admission
  initialization with its refusal reasons (§1.5), the session binding read in
  `respawn_after_death`, and the removal of the `None` placeholder. If KILL-REWARD-COMP-1 merged
  first, its `no_progression_binding` arm reads the principal's session binding; otherwise
  KILL-REWARD-COMP-1 reads it when it merges.
- **Not in scope:** any migration, durable schema or receipt change, any change to
  `durability/character_progression.rs`, the level-dependent speed, and a backfill.
- **Acceptance:**
  - A PG case: a new Character (root revision 1) is admitted on a World with the section, its row
    is initialized with the eight stored §1.3 values, and a player death commits a durable death receipt
    whose `policy_digest` matches the content.
  - A PG case: a second admission of the same Character with the same content finds the matching
    row, calls no initializer and keeps the binding.
  - A PG case: a Character past root revision 1 with no row is admitted with no binding, logs
    `progression_unbound reason=progression_uninitialized`, makes no initializer call, and its
    death respawns on the non-durable path within one cadence tick with no durable write.
  - A PG case: a row initialized under one content, then admitted under a content with a changed
    table, is admitted with no binding, logs `progression_unbound
    reason=progression_context_mismatch`, and its death respawns with no durable write. A second
    case changes only the death policy and gives the same result.
  - A unit test: a World pin without the section gives no binding, and the death path is the
    current non-durable respawn.
  - A unit test that the binding's `N` is `CHARACTER_EXPERIENCE_TABLE_LEVELS` on both the death
    and the kill reward paths.

## 3. Rejected options

- **Build the policy from a formula at runtime.** `FiniteProgressionPolicy` is a finite oracle by
  design and fails closed outside its table. A runtime formula would also give no evidence
  revision to pin.
- **Store the thresholds in the progression row.** That needs a migration and duplicates
  content per Character. The revision and `policy_digest` already bind the row to the content.
- **Use the World's `sim_profile_revision` as the `simulation` revision.** Every unrelated
  simulation profile change would refuse all progression writes (§0.2).
- **Pin the revisions in `ArtifactExpectation`.** The outer expectation lists artifact
  revisions, not ruleset content. The native gameplay section already carries a digest-bound
  pin that the runtime decodes.
- **Hand-numbered revisions.** A table edit that forgets to bump its revision would match old
  rows with new thresholds. Content-addressed revisions cannot drift.
- **A shorter table (for example 1000 levels).** A Character reaching the end would need a new
  table revision, which refuses every existing Character (§1.6 B). 2000 levels are out of
  practical reach.
- **Initialize lazily at the first award.** The durable layer admits a row only at root
  revision 1. The first award usually comes after other Character writes, so a lazy start
  would leave most new Characters without progression.
- **Backfill Characters past revision 1 now.** That changes the durable initialization rule and
  needs persistence review. With no production Characters it has no current user (§1.6 C).

## 4. Decision test

1. **Must it be decided now?** Yes. PROGRESSION-OWNER-1 cannot start without a content source,
   and live XP, durable player death and the Bestiary all wait on it.
2. **What is blocked?** Live kill XP and Bestiary (KILL-REWARD-COMP-1 §1.5), the durable player
   death (DEATH-2), and every level-dependent rule.
3. **What becomes harder later?**
   - The `progression` section is part of the native gameplay pin format (§1.6 A).
   - The eight stored revisions, `policy_revision` binding the death policy, are durable in
     every progression row and receipt (§1.6 B).
   - `CHARACTER_EXPERIENCE_TABLE_LEVELS` fixes `N` across the death and kill reward paths.
4. **What would justify superseding it?**
   - A progression migration owner, which would allow content changes for existing Characters.
   - A measured cost of the per-award table clone.
   - Reference evidence for the low-level bonus or another declared difference.
5. **What is deliberately not decided?** The progression migration, a backfill, level-dependent
   speed, skills and magic level progression, stamina, experience boosts, party sharing and
   quest experience content.
