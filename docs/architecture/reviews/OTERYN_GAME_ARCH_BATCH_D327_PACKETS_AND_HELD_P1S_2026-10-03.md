# Architect batch: worker packets (D327), #1438 repair decision, #1625 coverage

```yaml
decision_id: ARCH-BATCH-D327-PACKETS-V1
status: CANDIDATE
date: 2026-10-03
owner: Sol Supervising Architect
requested_by: control plane (D321/D326 1b batch)
writes_on_other_prs: none
```

This bundle contains:

- §1: three accepted worker packets. These are ACH-NOTIFY-2 (D327), CHARM-DESC-A2 (#1635) and one playable-path slice, CHAR-REV-SEQ-1.
- §2: the repair decision for the P1 held on #1438.
- §3: the coverage statement for the deferred P1 on #1625.

The bundle changes no code, no contract and no wire. Live PR and Issue state governs. The dependency notes below record the state when this was written.

## 0. Leases and serialisation

None of the three packets needs a new capability, command, state domain or migration:

- ACH-NOTIFY-2 reuses ACH-NOTIFY-1's cap 8 and domain 13.
- A2 and CHAR-REV-SEQ-1 change runtime and durability only.

The next free numbers stay at **cap 11 / cmd 15 / domain 14 / migration 0054**, as the control plane holds them. A worker that finds it needs a number stops and asks the control plane. It does not take one itself.

Owned paths overlap in one place: `gameplay_transport/mod.rs` is touched by both ACH-NOTIFY-2 and A2. The order is fixed:

| Order | Packet | Starts when |
| --- | --- | --- |
| 1 | ACH-NOTIFY-2 | #1653 (ACH-NOTIFY-1) has merged |
| 2 | CHARM-DESC-A2 | #1652 (CHARM-DESC-FENCE-1) and #1651 (the CHARM-DESC-FENCE-LEASE lifecycle, D324) have merged, **and** ACH-NOTIFY-2 has merged or released `gameplay_transport/mod.rs` |
| parallel, then 3 | CHAR-REV-SEQ-1 | authoring starts now on disjoint paths; the integration phase (§1.3) waits until A2 has merged and released `gameplay_transport/mod.rs` |

If CHAR-REV-SEQ-1 finds that it must edit a path that another packet owns, it does not edit that path. It reports the call site and waits. The paths concerned are `gameplay_transport/mod.rs` (except the §1.3 integration phase), `actor_spell.rs`, `ability/commit.rs`, `foundation/runtime_actor_carrier.rs` and `interaction/chest_use.rs`.

## 1. Worker packets

### 1.1 ACH-NOTIFY-2 (D327)

```yaml
task_id: OTV2-20261003-ach-notify2
worker: oteryn-hard-worker   # session-scoped delivery and fail-closed disconnect
review: protocol and persistence review (Codex, final frozen head)
branch: claude/ach-notify2-20261003
base: main after #1653 merges
owned_paths:
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - crates/protocol-oteryn/src/achievement_notices.rs
  - crates/protocol-oteryn/src/achievement_notices_tests.rs
  - docs/agents/tasks/archive/OTV2-20261003-ach-notify2.md
leases: none (cap 8 / domain 13 are ACH-NOTIFY-1's)
depends_on: [#1653]
```

Scope (D327). Nothing else is in scope.

1. **Tri-state `UseOutcome.earned`.** Replace the current two-state value with three explicit states:
   - `Earned(notices)`: the grant committed with `Granted`, and the transaction read the notices.
   - `NoneEarned`: nothing was granted, either because no grant applied or because the outcome was `AlreadyHeld` or `Retired`.
   - `Unknown`: the grant committed, but the notice read failed or overflowed.

   No caller may read `Unknown` as `NoneEarned`.
2. **Bounded in-transaction read.** The notice read inside the grant transaction is limited to `catalogue_len + 1` rows, where `catalogue_len` is the size of the loaded Achievement catalogue. If the read returns more than `catalogue_len` rows, the state is invalid. The result is `Unknown` and the overflow is logged as a defect. The read is never truncated silently.
3. **Disconnect when cap 8 is selected.** If the session negotiated `ACHIEVEMENT_NOTICES_V1` (cap 8), then an `Unknown` outcome, or a failed post-commit delta send, closes the connection with the existing fail-closed disconnect path. The client resynchronises from the domain 13 snapshot on resume (`resume.rs`). Without cap 8, nothing is sent and the outcome does not disconnect. The durable grant is never rolled back because a notice failed.
4. **Tests.** Each of these needs a test:
   - each of the three states;
   - a read of exactly `catalogue_len` rows passes, and a read of `catalogue_len + 1` rows gives `Unknown`;
   - a disconnect with cap 8 selected, and no disconnect without it;
   - a resume after the disconnect delivers the snapshot with the grant;
   - the Postgres case in `chest_use_postgres_cases.rs`.

Validation:

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p protocol-oteryn
cargo test -p game-server
python3 tools/agents/validate_governance.py
```

The game-server Postgres cases run under repository CI.

### 1.2 CHARM-DESC-A2 (#1635)

```yaml
task_id: OTV2-20261003-charm-desc-a2
worker: oteryn-hard-worker   # session-generation fencing, runtime authority
review: concurrency, persistence and combat review (Codex, final frozen head)
branch: claude/charm-desc-a2-20261003
base: main after #1652, #1651 and ACH-NOTIFY-2 merge (see §0)
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/ability/charm_desc_fence_gate_tests.rs
  - apps/game-server/src/durability/fresh_admission.rs   # call sites only
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-a2.md
leases: none (no wire, no migration)
depends_on: [#1652, #1651, ACH-NOTIFY-2]
closes: #1635
```

Scope: D295 item 4 (option A2) together with the CHARM-DESC-FENCE-LEASE lifecycle (D324, `OTERYN_GAME_CHARM_DESC_FENCE_LEASE_DECISION_2026-10-03.md`, #1651).

1. **Admission.** At admission, bind the attacker's `CharacterLease` to the runtime player slot. Inside the FND-04B recovery and takeover atomic boundary, session replacement rebinds the slot in place. The rebind drops the old fence atomically.
2. **Fence lifecycle.** The lifecycle has three steps (D324):
   - Fence the slot under the runtime lock, with no I/O while the lock is held.
   - Commit the durable transition.
   - Settle the slot according to the outcome.

   Lifting a fence requires the exact token. The runtime lock is never held across durable I/O.
3. **Same-session continuation (FND-04B §22).** Reconcile first:
   - If the state is terminal, do not continue.
   - If it is non-terminal and the binding is proven, bind unfenced.
   - If the binding cannot be proven, fail closed.
4. **Live attacker authority.** `commit_exact_owner_damage`, `_primary_damage` and `_charm_damage` take their attacker authority from the bound slot. The #1652 gate test (`charm_desc_fence_gate_tests.rs`) is relaxed in the same change. It must still fail any production caller that does not go through the bound slot. It is not deleted.
5. **Tests.**
   - A rebind during an in-flight charm or descriptor damage commit settles exactly once.
   - A stale token cannot lift the fence.
   - Every branch of the reconcile has a case.
   - The gate test fails on a synthetic unbound caller.

Validation: the same as §1.1. Add `cargo test -p game-server ability::` and the fresh-admission Postgres cases.

### 1.3 CHAR-REV-SEQ-1 (playable-path pick)

**Why this slice.** Every next playable-path progression system waits on CHAR-REV-SEQ-1:

- QUEST-STATE-1 (quest progress, with NPC and quest content behind it);
- STANCE-1;
- PREY and Task Board writes;
- BOSSTIARY-1;
- CYC-DISCOVERY-1.

It needs no content, no wire and no migration. It also removes a live defect class. On `main` today, revision-advancing writers can race each other, and a race surfaces only as `CharacterRevisionMismatch` failures (QUEST-STATE-0 §5.2).

A content slice such as CYC-CONTENT-1 is left out, because the control plane asked for non-content workers. A second slice is deferred until A2 has released the shared transport paths.

```yaml
task_id: OTV2-20261003-char-rev-seq1
worker: oteryn-hard-worker   # persistence and concurrency
review: persistence and concurrency review (Codex, final frozen head)
branch: claude/char-rev-seq1-20261003
base: main (now)
owned_paths:
  - apps/game-server/src/durability/character_revision_sequencer.rs   # new
  - apps/game-server/src/durability/mod.rs                             # module registration only
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/durability/bestiary_progress.rs
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/combat/death_reward.rs                        # D336: XP -> Bestiary composition
  - apps/game-server/src/durability/monk_state.rs
  - apps/game-server/src/durability/character_build.rs               # D356: build writer
  - apps/game-server/src/durability/character_proficiency.rs         # D356: proficiency writer
  - apps/game-server/src/gameplay_transport/charm.rs
  - apps/game-server/src/gameplay_transport/charm_native.rs
  - apps/game-server/src/gameplay_transport/charm_native_tests.rs
  - apps/game-server/src/gameplay_transport/monk_save.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs       # bind parameter
  - apps/game-server/tests/support/character_build_postgres_cases.rs        # D356
  - apps/game-server/tests/support/character_proficiency_postgres_cases.rs  # D356
  - apps/game-server/tests/support/combat_bestiary_postgres_cases.rs
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs
  - apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs   # new
  - apps/game-server/tests/durability_postgres.rs                      # one mod line
  - apps/game-server/src/gameplay_transport/mod.rs                     # integration phase only (D336), after A2 releases it
  - docs/agents/tasks/archive/OTV2-20261003-char-rev-seq1.md
leases: none (runtime cursor; no table, no migration)
depends_on: [QUEST-STATE-0 decision (accepted)]
```

Scope: QUEST-STATE-0 §5.2, "One write in flight per Character", with nothing added.

1. **One sequencer per Character.** It is owned by the owning channel runtime and holds the revision cursor. The cursor advances only after a committed receipt.
2. **Writers move onto the sequencer.** Every revision-advancing writer on `main` moves onto it:
   - XP;
   - death;
   - Bestiary;
   - charm, with its in-transaction fee burn;
   - monk state save;
   - build and proficiency (D356, QUEST-STATE-0 §13.1).

   A writer that advances the revision outside the sequencer is a defect. Where it can be done structurally, a test enforces this, like the #1652 gate test.
3. **Compositions hold the slot.** A composition holds the slot for its whole chain. For example, in a creature death, Bestiary takes the revision that XP committed. Other requests wait for the slot.
4. **Mismatch handling, unchanged from §5.2.**
   - Where the binding excludes the revision (Bestiary), reload the cursor and retry once.
   - Where the binding includes the revision (XP, death, charm, monk, build, proficiency), fail closed and report a defect. Do not retry.
5. **Locks.** The sequencer slot is an asynchronous per-Character queue. It is **not** the runtime lock. The runtime lock is never held while waiting for the slot or across durable I/O (D324).
6. **Session replacement.** A replaced session's queued writes are refused at the existing session-generation fence. The sequencer adds no second fence, so session-generation fencing is unchanged.
7. **Out of scope.** STANCE-1, QUEST-STATE-1 and the PREY writers are built on the sequencer later. They are not part of this task.

Tests:

- concurrent XP and charm on one Character commit in sequence, with no mismatch;
- a death chain of XP then Bestiary;
- the Bestiary retry-once path;
- XP, death, charm and monk each fail closed with no retry;
- a bypass writer is caught by the structural test.

Validation: the same as §1.1, including the durability Postgres cases.

**Integration phase (D336).** The worker authors the sequencer, the writers and the tests now.

- The sequencer's runtime home is a `revision_sequencer` field in `ComposedFreshAdmission`.
- Its initialisation in `gameplay_transport/mod.rs`, about two lines, is added only after A2 has merged and released that path. The worker brings in A2 by merging `main`, adds those lines, and then freezes. The freeze comes only after that edit.
- `foundation/runtime_actor_carrier.rs` is not touched.
- In `combat/death_reward.rs`, the death composition holds the sequencer slot for the whole XP → Bestiary chain.
- Monk save fails closed on a mismatch, as §5.2 requires.

Any other call site in a path that another packet owns (§0) is reported, not edited.

## 2. #1438 held P1 (comment 4172937101): imbuement research data repair

**Finding.** In `tools/content-schema/imbuement-authoring/samples/global-research-closure.json`, 32 sources carry full captured pages in `captured_text`, up to about 64 KB each. The repository therefore redistributes third-party page bodies, although the closure needs only the quoted passages. Across all sources, the quotes total at most 677 characters per source. The largest single quote is 413 characters.

**Decision.** Repair the data and the validator in one push, on #1438's own branch, by its writer. This happens only after the D319 queue drains. No architect writes go to #1438.

1. **`captured_text` becomes a bounded excerpt.** It contains only the source's quoted passages, in document order, each whitespace-normalised and joined by `" … "`. Set `scope` to `"Bounded excerpt: quoted passages only, whitespace normalized"`. Recompute `captured_text_sha256` over the new excerpt.
2. **Keep the provenance.** `original_digest` (the exact response bytes) and the retrieval metadata stay unchanged. They remain the link to the full page and need no redistribution.
3. **Sources with no quote.** There are five, including `wiki_br_current_timers`, `qa24514`, `vibrancy_original_example_revisions` and `trends_current_pz_pause`.
   - If no claim or fact references the source, drop it.
   - If something references it, keep a locator excerpt of at most 300 characters that identifies the passage, and give that excerpt the same `scope` wording.
4. **New validator bounds in `research_closure.py`.**
   - `captured_text` is at most 1,000 characters per source.
   - Each joined passage is at most 450 characters.
   - Every quote, `literal_quote`, `claims[].quote` and `facts[].quote_refs[].text` is still a substring of its source's excerpt. The substring rule is unchanged.
   - `captured_text_sha256` matches.
   - Add a negative test for each bound.
5. **Acceptance.**
   - The existing closure tests pass unchanged.
   - The file shrinks to the excerpts.
   - No claim loses its quote support.

   The P1 is closed by the push that does all of the above. Fixing it does not require deleting a claim.

## 3. #1625 deferred P1 (comment 4173012827): coverage

The P1 is a live attacker-authority gap: charm and descriptor damage can commit against an attacker whose session was replaced.

| Part | Covered by | State |
| --- | --- | --- |
| The rule and the gate (`commit_exact_owner_damage` / `_primary_damage` / `_charm_damage` have no unbound production caller) | D295 (#1638) | merged |
| The fence lifecycle across replacement and continuation | D324 (#1651) | frozen, in review |
| The binding implementation, A2 | #1635, packet §1.2 | packet accepted here |
| **The structural gate test CHARM-DESC-FENCE-1** | **#1652** | **open; held in its third P1 round** |

**Statement.** The P1 is covered by D295, D324 and #1635, except for **one remaining gap**: until #1652 merges, nothing structural stops a new production caller of the three commit functions. While #1652 is open, any PR that adds such a caller must be rejected in review. The #1625 P1 is resolved when both #1652 and #1635 have merged. The remaining gap is tracked on #1652 and nowhere else.
