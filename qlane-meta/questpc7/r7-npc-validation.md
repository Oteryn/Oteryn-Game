# QUESTPC run 7 NPC validation

Task: QUESTPC run7 CODER slot4. Evidence below is from local, uncommitted task work in `/home/mole/oteryn-cache/rs-wt/questpc`; it is not repository CI, a publication claim or whole-quest certification. No new code or tests were added for this evidence note. Base HEAD is `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`; the commands exercised that checkout plus its uncommitted task changes.

## Final retained scope

The runtime catalogue admits only the newly owned Order of the Stag program: 99 source rules and six NPC keys. The storage owner executes its supported branches; this does not qualify every branch or the complete quest. Runtime filtering happens before native track refinement, definition/hash extension, planning and dispatch. All 13 source documents remain schema validated: 1,123 rules, 230 track declarations and 78 unique NPC keys. Their declared defaults are 180 tracks at `-1` and 50 at `0`.

The retained route includes the NPC talk ingress, exact actor/conversation checks, existing Character revision sequencer and session-generation fence, locked storage guard reevaluation, atomic quest receipt/state/track write, same-transaction bounded response proof and receipt reconciliation. Item, money, XP, outfit and other effects requiring another native owner refuse the complete plan. Pure replies/topic changes create no Character revision or quest receipt.

The twelve existing quest definitions are not activated/refined by this route. Father's eleven-track acceptance is not mounted. `MAX_NPC_TRACK_CHANGES` and the general quest receipt bound remain eight; the provisional eleven-track exception was removed from both code and the root-owned migration.

## Final unit command

```sh
QPC_SLOT=4 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-quests --lib npc_rules -- --nocapture
```

Observed final output:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 1.97s
running 25 tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 234 filtered out; finished in 0.19s
```

This includes:

- `every_registered_source_program_passes_the_closed_schema`;
- `npc_activation_preserves_all_twelve_legacy_quest_pins`;
- `storage_owner_refuses_father_until_a_durable_definition_migration`;
- the unchanged `father_acceptance_refuses_all_eleven_atomic_writes_without_splitting`;
- `storage_owner_refuses_inventory_dependent_fallbacks` and `storage_owner_preserves_order_and_rejects_unowned_effects`;
- existing source-order, keyword, subtype, arithmetic/consumption, clock, custody and native-resolution checks.

The legacy-pin regression compares every one of the twelve old quest hashes and every old track definition against the catalogue before NPC extension. It also checks the full source default counts and that only one runtime program remains admitted.

## Actual pin-regression red, restoration and green

For the red check only, the owned `NpcRuleCatalog::runtime` retain predicate was temporarily changed from the Stag quest predicate to `.retain(|_| true)`. No shared file was changed for this experiment. The Stag predicate was restored immediately after the failed run, before the green run and final PostgreSQL validation.

Both runs used this exact command:

```sh
QPC_SLOT=4 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-quests npc_activation_preserves_all_twelve_legacy_quest_pins -- --nocapture
```

Red: build **1.90s**, test **0.24s**, exit **101**, **0 passed / 1 failed**. Exact assertion failure:

```text
assertion `left == right` failed: unfinished legacy quest pin: oteryn:quest.bigfoot_s_burden_quest
```

The observed old hash was `f4aea058bd9f28e4c1f12b44413cc4f37ad6ad8488246bc66626f8d7b497ddc8`; broad activation produced `080eae05b7a6ecb63dc8fabd84ced669ecf031467f3aca93f11069fd5ef2118c`.

After restoring Stag-only activation: build **2.21s**, test **0.27s**, exit **0**, **1 passed / 0 failed**. The final 25-test command above also passed this regression.

This is an observed loader/hash regression, not a claimed PostgreSQL replay of all twelve legacy quest histories. The concrete durable consequence follows from `crates/gameplay/durability/src/durability/quest_state/writer.rs:377`: a stored unfinished state whose hash differs from the current definition returns `QuestRefusal::RevisionMismatch`. A completed state keeps its old pin. Broad activation was withdrawn rather than bypassing this check or repinning stored state.

## Final real PostgreSQL command

```sh
QPC_SLOT=4 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-dispatch --lib catalogue_npc_source_acceptance_is_atomic_and_replay_bound -- --nocapture
```

Observed final output:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 24s
running 1 test
POSTGRES_CASE_COMPLETED catalogue_npc_source_acceptance_is_atomic_and_replay_bound 51.6817523s
test gameplay_transport::monk_save::postgres_tests::npc_program::catalogue_npc_source_acceptance_is_atomic_and_replay_bound ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2493 filtered out; finished in 51.68s
```

The exact test file is `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_program.rs`. It drives the real NPC command path using placed Kesar and Demeron actors, the production content/runtime fixture and PostgreSQL migration set. It proves:

1. Kesar's first `hi` enters the real conversation owner. `help` before the Lion prerequisite creates no Stag track.
2. The fixture establishes prior Lion stage 4 and then 5 through its existing native quest writer. This is prerequisite setup, not certification of the complete playable Lion quest route.
3. Kesar `help` rechecks the foreign Lion `>=5` and own Stag `<1` guards in the locked owner and commits Stag `-1 -> 1` as one track/receipt and one Character revision.
4. Exact receipt recovery returns the original reply; a different message on the same occurrence conflicts.
5. A fresh occurrence under a stale Character lease generation is rejected, and the durable copy remains unchanged.
6. Demeron's first `hi`, distinct `<4` mission guard, `tell` and three `yes` topic steps run through real dispatch. These dialogue steps create no extra durable mutation or invented proof-item consumption.
7. The committing Kesar source branch has exactly one durable receipt.

The final success was not a route-less skip: the completion marker above was emitted after the PostgreSQL scenario returned successfully.

## Earlier failed qualification attempts

Earlier dispatch compilation attempts used the same test filter, initially without `--lib` and subsequently with `--lib`. They did not run the NPC case and produced no completion marker. Observed blockers were other lanes' evolving code: Falcon coordinate types, Falcon SQL-safe dynamic fixture SQL, Newhaven loot-table tuple/signature changes, Newhaven SQL-safe string type inference, and protocol/domain CharacterId comparison. Those owners repaired the callers before the final passing run. No separate build/test duration was printed for these failed compilation attempts.

One earlier actual PostgreSQL attempt, before the Newhaven artifact repair, used the final command above. It built in **6m 00s** and failed in **17.36s**, **0 passed / 1 failed**, before reaching NPC setup:

```text
Error: "Content(InvalidArtifact(\"Reference Item Equipment vocation order\"))"
```

The Newhaven owner corrected item 7991's vocation order to `DRUID,SORCERER`; root regenerated the native artifact. The final passing command ran after that repair and the six Stag greeting book entries were integrated.

## Concrete legacy/Father blockers and owning contracts

- **Legacy twelve:** activating all thirteen source programs changes existing quest definition hashes, as the actual red experiment above demonstrates. An unfinished stored old hash therefore reaches the existing writer's `REVISION_MISMATCH` refusal. No accepted audited migration or forward/rollback compatibility evidence for those old states was produced in this lane. The final code preserves their old tracks and hashes and filters them from this new runtime owner.
- **Father:** the exact source acceptance sets eleven distinct tracks in one source operation. Both `plan` and `plan_storage` return `NpcRuleError::LimitExceeded` under the preserved eight-track bound, with unchanged input facts. The final unit suite explicitly asserts this refusal and that Father's program is absent from the runtime catalogue. No Father PostgreSQL completion or eleven-track durable operation is claimed. Its activation also falls within the legacy definition migration problem above; splitting the operation or globally raising the cap is not a repair.

Owning contracts:

- `docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md`, **§6 Revision pinning**: a different definition hash is `REVISION_MISMATCH` until a DUR-04 migration for that quest exists; completed states retain their pin. **§4** also requires the hash match for an unfinished state before committing a transition.
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`, **§11 Staging, activation and rollback**: durable migration requires forward and rollback compatibility proof before production activation.
- The same DUR-04 contract, **§12 Durable-state migration classes**: changes affecting durable semantic interpretation require an explicit migration classification; explicit data migrations use accepted persistence/domain ownership, and normalization cannot bypass domain validation or create/destroy player value. **§23, evidence item 19** requires migration-required content to remain inactive until migration proof exists.

No migration authority, direct repin or global cap exception is inferred from source parsing or fresh-character qualification.

## Hygiene and handoff

The owned-path `git diff --check` command returned exit 0, with no output. Rust formatting was applied only to the owned Rust files. Root owns the shared hooks, migration 0969, generated content, combined Clippy, final independent review and publication decisions. No Git writes, credential reads, dependency downloads or external publication were performed by this lane.
