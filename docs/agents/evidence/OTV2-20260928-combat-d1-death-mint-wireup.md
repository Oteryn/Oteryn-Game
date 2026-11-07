# OTV2-20260928-combat-d1-death-mint-wireup evidence

Task: `docs/agents/tasks/active/OTV2-20260928-combat-d1-death-mint-wireup.md`.
Allocation: #162 comment 5873329796 ("Allocation: Combat D1"). Admission `main@e1ccfd4`.
Machine-readable companion: `OTV2-20260928-combat-d1-death-mint-wireup.json`.

D1 is a test-only split. It is not D admission and makes no playable-Combat claim.

## API added

| Item | Location | Visibility | Notes |
|---|---|---|---|
| `CreatureDeathOccurrenceKey` | `foundation/runtime_actor_carrier.rs` | `pub(crate)` | Private `ActorRef` newtype. It has typed getters `world_id`, `channel_id`, `scope_ownership_generation`, `actor_local_id` and `actor_local_generation`. There is no constructor. |
| `CreatureDeathOccurrenceRef::death_key()` | same | `pub(crate)` | This is the key's only source: a committed lethal occurrence of the physical owner (decision §4.1). |
| `ItemMintCause::from_creature_death(key, TypedDefinitionRef, String, u32)` | `durability/item_mint.rs` | `pub(crate)`, `allow(dead_code)` | The full typed tuple, with no caller death bytes (§4.2). `for_test` is kept. |
| `CombatDeathFixture` | `foundation/runtime_actor_carrier.rs` | `cfg(test)` | One-creature carrier. A death occurs only through the owner commit and Combat's `project_fixed_one_creature_death`. |
| `d1_*` wire-up helpers | `tests/support/item_mint_postgres_cases.rs` | test only | Death → one `VSL_COMBAT_FIXTURE_PROFILE` loot entry (draw 0, quantity 1) → `freeze_item_mint` / `commit_item_mint` under the live fence. |

Production code does not call the wire-up. No runtime path is activated.

## Acceptance tests

| Row | PostgreSQL case | Unit case |
|---|---|---|
| (a) one death → exactly one MINT | `d1_one_creature_death_mints_exactly_one_item` | — |
| (b) replay / re-freeze → same item, same IDs, no duplicate | `d1_replayed_or_refrozen_death_resolves_to_the_same_item` | `committed_lethal_death_key_is_the_exact_owner_actor_ref_and_replay_stable` |
| (c) stale generation after a scope move is refused (D52) | `d1_stale_generation_death_is_refused_after_the_scope_moves` | `only_a_committed_projected_lethal_occurrence_yields_a_death_key` |
| (d) despawn → no death, no MINT | `d1_administrative_despawn_produces_no_death_and_no_mint` | `only_a_committed_projected_lethal_occurrence_yields_a_death_key` |
| (e) two creatures → distinct keys and items | `d1_two_creature_deaths_have_distinct_keys_and_items` | `two_fixture_creature_deaths_have_distinct_death_keys` |

Row (c) sets up the pending MINT in this order:

1. The death is frozen in generation 1.
2. The scope is replaced to node 2, which makes generation 2.
3. Commit and re-freeze both return `AuthorityRejected`, on both holders.
4. The stale carrier returns `WrongScope`.

In the end one reservation remains, and zero items, receipts or outbox rows exist.

## Validation (local, PostgreSQL 17.11, `server_version_num=170011`)

- `cargo +1.94.0 fmt --check`: pass.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked --workspace`: see the JSON companion.
- `item_mint_postgres`: 478 passed, 0 failed. The 5 D1 cases each log
  `ITEM-MINT-PG: database im_d1_* on server_version_num=170011`.
- `durability_postgres`: 554 passed, 0 failed.
- `character_authority_postgres`: 483 passed and 8 failed. All 12 `item_mint_postgres_cases` pass (7 stage C and 5 D1). The 8 failures are the known pre-existing hard asserts that require `server_version_num == 170006`; the local server is 17.11. The same 8 are listed in the stage C evidence:
  - `authenticated_intent_qualification_matrix`
  - `bootstrap_is_atomic_and_audit_is_published_held_and_expired`
  - `character_progression_postgres_cases::distinct_occurrences_with_one_predecessor_cannot_both_commit`
  - `character_progression_postgres_cases::exact_replay_reconcile_and_restart_readback_are_durable`
  - `character_progression_postgres_cases::rollback_concurrency_and_ended_node_preserve_single_revision`
  - `character_progression_postgres_cases::stale_fences_context_and_missing_state_fail_closed`
  - `fresh_store_admission_refuses_any_prior_character_row`
  - `fresh_store_rerun_is_idempotent_and_interpretation_waits_for_it`
- `tools/agents/validate_governance.py`: pass.
- `tools/agents/validate_inherited_prompt_policy.py`: could not complete locally. The unauthenticated `api.github.com` fetch of the private META policy returns HTTP 403 in this session. This is an environment limit, not a finding, and it also happens without this diff. Repository CI runs this validator with a token.

## Findings

- The fixture carrier admits one creature per Channel generation, and a retained corpse blocks replacement. Row (e) therefore uses two assigned Channels of the same World on one node.
- The loot entry, item, revisions and Ground placement are `VSL_COMBAT_FIXTURE_PROFILE` test values: the corpse position plus a fixture encoding of the death key. No probability, loot table, resource number or placement rule is decided.

## Not proven

- Runtime activation.
- XP.
- Real loot tables.
- The Character progression readiness proof.
- The VSL §19 rows.
- The canonical PostgreSQL 17.6 lane.
