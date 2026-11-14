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

## Recovered execution evidence on the published d583f02a head

The renewed owner-PC continuation inspected the complete archived logs for
run **43535792273**, bound to `d583f02a2a158097ec55c09f37b6c3499e1e6f59`.
Both build legs and all **24 PostgreSQL partitions passed**. Exact executed
new tests, not merely names from a list:

- Partition 20: `all25_native_dodge_cancels_real_source_spells_mana_and_attached_conditions`,
  PASS, 24.461 s, 2026-11-14T01:00:31Z.
- Partition 16: `all25_native_dodge_cancels_melee_before_block_and_retains_replay`,
  PASS, 3.900 s, 2026-11-14T01:02:12Z.
- The actual PostgreSQL charm consumer tests for Cleanse, Fatal Hold, Numb, Carnage,
  Cripple, Bless, Adrenaline, Void Inversion and Parry passed in those partitions.
- Both partial/full-harmony Outburst cases passed; the previously failing
  `player_death_nested_maximum_501_item_reset_makes_bounded_atomic_progress`
  also passed (partition 18, 48.328 s). This run does not establish the root
  cause of that older failure.

The overall workflow was NOT green: its separate **Node boot** job
132264343834 failed with 13 failed rows / 12 root groups, including
S16a/oressa-reachable, W20b/light-cast-mana, S13a/solo-refusal,
S13b/solo-zero-charge, S08/earned-levels, P21/player-acquisition,
P19/influenced-selection-combat and C20d/away-from-temple. The full log is
retained outside the source tree and is not treated as a charm-test failure
or waived as an infrastructure flake.

The d583f02a publication and test results are therefore confirmed. This
supersedes only the earlier missing-execution and publication uncertainty,
not the open G1/G3/G4 mechanisms or final exact-candidate review requirement.

## Existing Cleanse/Hex dependency composition

The owner-PC continuation forward-merged current main at
`3e9e3eb8756acec917b92101de42ba50dca45355` and exact #3176 head
`8382ce41514c415996216d85d4d12a9e9a0e8070`, without modifying either remote
branch. Cleanse now uses that existing single-selection canonical/player
condition-store composition and its Hex immunity tests, rather than another
independently reimplemented cleanse. Local composed-owner verification is
recorded separately from d583f02a's earlier CI. No deployment or main merge.

## Incoming-order implementation and independent PC verification

The recovered incoming patch is now integrated on the lead branch after the exact
main and Cleanse/Hex dependency merges. Lead compared all 17 Rust file contents
byte-for-byte with the isolated verification workspace at the same committed base
`e2be8f9d0bcec4c6c1c2da9d72b0027302712147` and reran the tests personally.

Implemented: raw pre-player-defense Parry input; post-mitigation, pre-shield hit
gate; atomic existing physical-source/player-vitals composition; major/minor
selection in accepted category and source-liveness order; defensive source ticks
with exact provenance, no-source and Agony exclusion; native/composed Dodge; and
retained source replay after a Parry kill. Native melee retains its actual held
session behavior. Cleanse uses the imported canonical/player single-selection
Hex implementation rather than another condition store.

Independent exact-source results (absolute PC cargo wrapper, local Docker PG):

| Filter | Passed | Failed | Evidence scope |
|---|---:|---:|---|
| `incoming_order_` | 6 | 0 | actual source owners and refusal/replay |
| `owned_source_tick_tests` | 14 | 0 | existing native tick owners |
| `actor_charm_cleanse_tests` | 2 | 0 | composed canonical/player Cleanse/Hex |
| `all25_native_dodge_` | 2 | 0 | actual native spells/melee |
| `srx2_charm` | 10 | 0 | real PostgreSQL and combat consumers |

The PostgreSQL output contains actual database transaction events; no missing-route
early return is counted. The combined 34 cases are focused checks, not a full
workspace total. Strict `clippy --all-features --all-targets -- -D warnings` passed
on the canonical source for combat/entities/progression/spell/dispatch. Both
format checks, diff and governance passed; governance unittest suite: 129 OK.

Two old assertions were updated to the accepted incoming-order contract, not
relaxed: mana-only Numb/Adrenaline now require exactly zero speed contribution;
Bless/Parry requires no source HP mutation during read-only reflection preparation
and the same source death after the actual joined hit. The underlying positive
health-hit minor tests and lethal/death replay assertions remain.

Remaining before full completion: outgoing direct/native/rune effects including
Cripple's canonical condition bridge, secondary-auto critical consumer, delayed
source integration, final exact-head independent review and repository CI/Node
boot. This commit is a meaningful implementation increment, not task closure.
