# Charm runtime completion — native Dodge increment

Task `OTV2-20261113-charms-leech-evidence`, PR #3183. This is an incremental
AUTHORING record, not a declaration that every charm or boundary is complete.
Source predecessor: `b7c5c14778abf342b7d3f5dc0d1e986fcbdba17c`.

## Runtime authority and reproduced defect

The accepted `CHARM4-INCOMING-EFFECT-ORDER-V1` decision in
`docs/architecture/reviews/OTERYN_GAME_CHARM4_INCOMING_EFFECT_ORDER_DECISION_2026-10-03.md`
requires a successful Dodge to cancel the whole incoming attack before mitigation,
reflection, HP, explicit Mana Drain and attached conditions. It does not establish
new live Tibia Test Server measurements. The external reference cut is unchanged.

PROVEN: the existing CreatureBite consumer did not cover the separate native
`DamageSpellOwner` and `AutoAttackOwner` routes. Before the fix, the actual native
First Mate Ratticus source spell killed a player with 185 HP despite a guaranteed
Dodge selection. The real-owner regression printed `Dodge must cancel HP, MP and
attached conditions` and failed before any production changes on the owner PC.

The PC source included only the disjoint #3184 main delta beyond the published
b7 predecessor. A fresh file-diff check between `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`
and `2acd6979f5522559382b50f2da812493a01ba3d9` confirmed no changes in the four
production files of this increment. After the PC went offline, its unreturned
green run and outgoing worker results were not assumed successful.

## Implementation

`PlayerLethalVitals::dodge_source_attack` is a read-only, default-false selection
seam. `CharmSourceVitals` rechecks the current player, session, pending-write and
reservation state, actual wild source policy, exact character/race assignment and
deterministic occurrence. No client-supplied percentage or synthetic HP receipt.

`DamageSpellOwner::commit_prepared` calls it before reflection, invisibility
dispel and all HP/MP/condition consumers. The real source cast retains a successful
no-effect target outcome in its normal occurrence memo. `AutoAttackOwner` does the
same before shield-block consumption, reflection and attached effects. Its
ordinary no-condition damage path now keeps the qualified source reference.

These owners already admit source identity, content, scope, generation and target
facts. This change does not grant authority from a race string or a historical
receipt, change a binding format, create another runtime owner, or add an
unguarded post-commit effect. Unassigned actors retain the default no-Dodge path.

## Actual-caller regressions

`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_incoming.rs`
is auto-mounted by the normal dispatch build. The tests are in-memory owner
scenarios, not PostgreSQL coverage. They use retained native content and the real
player/charm state adapter; only trigger probability is made certain by the
existing test helper, while effect shape and source identities remain canonical.

- Native physical/elemental spells, explicit Mana Drain, and condition-only
  source casts: no HP, MP, condition or player-revision change after Dodge.
- Native melee owner: requested damage is cancelled before a block/HP result.
- Both routes replay the same occurrence after the player's assignment changes
  to Parry; the already retained cancellation must not become a fresh hit.

Each full runtime scenario has its own 64 MiB test thread. No existing test is
removed, ignored, relaxed or given a larger gameplay timeout/resource budget.
The old `PlayerBiteVitals` production melee route is separate and remains in the
full incoming-order audit; this increment does not pretend to replace it.

## Verification custody

PASS: `cargo check --offline --locked -p oteryn-gameplay-combat
-p oteryn-gameplay-spell -p oteryn-gameplay-dispatch --all-features --all-targets`
on the VPS source tree, including compilation of the new tests (exit 0).

The first VPS full dispatch-test compilation exhausted its process virtual-memory
limit; a non-incremental attempt was stopped by the task-only memory guard above
10 GiB RSS. Neither attempt executed a test. This limitation does not justify
marking the regressions green or changing tests/CI. Runtime execution remains
required in the registered GitHub CI target or a recovered adequate development
machine. The pinned lockfile is unchanged; missing caches were populated only
with `cargo fetch --locked --target aarch64-unknown-linux-gnu`.

Additional exact results and the new candidate SHA are recorded in the PR after
completion/readback. Candidate-specific CI and independent review must refer to
that SHA, not the unpublished PC merge or the predecessor.

## Remaining completion work

Outgoing general charm hooks for the 12 native/ordinary player-spell effects,
Parry's raw pre-player-defence basis and ordered joint commitment, sourced DoT
ticks, remaining secondary/delayed/retry cases and #3176 Cleanse/Hex composition
remain open. The user request is not complete. No merge or deployment.

## Completed local checks before API publication

- All-targets compilation, including the new dispatch tests: PASS.
- `cargo test --offline --locked -p oteryn-gameplay-combat
  -p oteryn-gameplay-spell -p oteryn-gameplay-progression --all-features --lib`:
  **45 + 173 + 333 = 551 passed, 0 failed, 0 ignored**.
- `cargo clippy --offline --locked -p oteryn-gameplay-combat
  -p oteryn-gameplay-spell -p oteryn-gameplay-dispatch --all-features
  --all-targets -- -D warnings`: PASS (exit 0).
- Workspace formatting, explicit formatting of the auto-mounted incoming test
  file, `git diff --check`: PASS.
- Governance validator: PASS; governance unittest suite: 129 tests, OK.

The 551 runtime tests are the three named smaller libraries, not the new full
dispatch regressions. The latter have compiled successfully but still require
actual execution on CI. The PC RED, VPS resource-aborted full-test builds and
these passing checks are kept separate; none is relabelled as another result.
