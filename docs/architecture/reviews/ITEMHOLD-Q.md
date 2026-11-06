# ITEMHOLD-Q — quest Item catalogue holds

Base: `main` at `ed788c4bb`. Branch: `claude/happy-davinci-ijz3wy-itemhold`.
Scope is Item data admission, not quest runtime admission or starting-kit rewards.
Owner decision C19b keeps Newhaven as the starting-equipment source.

## Enforcement and repair

`content/items/definitions/items-*.json` stores `materializable`, `stack_class`
and typed evidence states. `tools/content-schema/reward-claim-authoring/reward_claim_authoring.py:122`
rejects an unmaterializable/unknown-stack definition, invalid nonstackable quantities
and unsupported stack maxima. Variant authoring emits `ITEM_SEMANTICS_MISSING`.
`tools/content-migration/quest_reward_item_semantics.py:157` derives the older
fluid/charge lowering holds. Those historical holds predate the native fluid,
timed-item and reward-bag owners now present on main.

The bounded `tools/content-migration/itemhold_q.py` overlay verifies the exact
Canary XML and client appearance digests, literal identity/name/weight/charge
facts, portability and stack/container/fluid flags. It rejects conflicting known
facts, completes nine canonical records, and adds only the two missing native
profiles (wolf/collar). Existing fluid, timed, starter and bag profiles retain
their owners. No validator predicate or source charge-disposition rule is relaxed.
The tree validator still requires exact equality with the qualified source tree.

Runtime inventory admission uses
`crates/gameplay/items/src/item_ref_admission.rs:31`: an exact active-generation
profile with inventory destination and a known stack class. A canonical flag
alone does not supply this runtime profile.

## Evidence and per-Item disposition

Canary XML below is retained byte-for-byte under
`imports/canary/items-xml/manifest.json:1` (SHA-256
`1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`).
It is `OTS_HYPOTHESIS_ONLY`, corroborated by admitted client flags and the retained
own-ID Wiki observations (`TIBIAWIKI_STRUCTURED`). The client is the digest-bound
15.30 appearance bank used by `quest_reward_item_semantics.py`.

The Wiki witness file
`docs/agents/evidence/OTV2-20261001-item-movable-source-qualification-v1.json:1`
is one JSON line. The witness keys below locate exact `raw_own_infobox` records
under `/competing_own_infobox_witnesses`; these are evidence, not new identity
allocations. Charges/Cup/Jug observations are in
`docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json:1`, selected
by `/admissions/*/item_id`; raw Wiki saves are in
`imports/tibiawiki/quest-reward-items/fandom-snapshot-r1.json:1` at their page IDs.

| Item | Result | Exact evidence and semantics |
| --- | --- | --- |
| 3572 Scarf | **RESOLVED** | `imports/canary/items-xml/items.xml:11286`: weight 200 centi-oz, armor 1, necklace slot, no charges/duration/capacity. Wiki witness `2473:0`: weight 2.00 oz, stackable no, armor 1, Neck; explicitly no special effects. NonStackable, non-container, no timer/charges. Existing `starting_loadout_profiles.py:1` supplies its exact native profile; no new kit grant is added. |
| 52732 Guardian Ring | **HELD** | Wiki witness `109358:0` proves weight 0.90 oz, armor 1, Finger, stackable no, level 0 and Newhaven reward. Identity already exists as `oteryn:item.tibia.i52732`; it is not missing from the canonical tree. The pinned XML has no effective 52732 row and the native provider has no profile. The retained Wiki infobox does not explicitly establish charge/decay/default attribute policy. Do not invent an OTS row or assert absent instance mechanics from omitted Wiki fields. A complete qualified donor/default-policy record is still required. |
| 3054 Silver Amulet | **RESOLVED** | `imports/canary/items-xml/items.xml:6946`: 200 charges, weight 500, poison/earth protection 10%, necklace slot; no decay/duration/capacity. Reward admission observation page 2829/revision 1194271 agrees on charges, weight and Neck. Preserve those exact existing equipment/protection facts; NonStackable, no timer/container, full charges 200. Existing timed host adapter `crates/gameplay/content/src/content/timed_item_definition.rs:57` resolves definition charges; no source tuple is reinterpreted as quantity. |
| 14156 Dog Collar | **RESOLVED** | `imports/canary/items-xml/items.xml:32492`: Noodles description and weight 110; no charges/duration/capacity. Wiki witness `57262:0`: exact item ID and weight 1.10 oz. Pinned client has take=true, no cumulative/container/unmove flag. NonStackable, no timer/charges/container; preserve accepted description. Add exact native profile using existing Crystal binding. |
| 12369 Captured Wolf | **RESOLVED** | `imports/canary/items-xml/items.xml:29995`: captured-wolf description, weight 5000; no charges/duration/capacity. Wiki witness `54281:0`: 50.00 oz and stackable no. Client proves portability/nonstackability. NonStackable, no timer/charges/container; preserve accepted description. Add exact native profile using existing Crystal binding. |
| 33330 Monolith Kit | **HELD** | `imports/canary/items-xml/items.xml:62867` says weight **750** centi-oz (7.50 oz); Wiki witness `92100:0` and canonical physical facts say **75.00** oz (7500). The appearance proves portability but cannot decide weight. Keep the hold and both conflicting observations; no silent factor-of-ten correction or construction-use policy is invented. |
| 39546 Primal Bag | **RESOLVED** | `imports/canary/items-xml/items.xml:73191`: weight 16000, no capacity/charges/timer. Wiki witness `98740:0`: 160.00 oz, stackable no. Client container=false: this is an opening reward token, not a container with guessed capacity. Preserve the existing reward-bag owner (`tools/content-migration/reward_bag_profiles.py:1`, retained `tools/content-schema/native-gameplay/reward-bag-profiles.json:1`). NonStackable, no container/charges/timer. |
| 44179 Bottled Dragon Breath | **HELD** | Canonical record `content/items/definitions/items-25500-25999.json:1` has UNKNOWN weight; pinned client object 44179 proves name/portability and a light flag, not physical weight or source defaults. No effective 44179 row exists in the retained Canary XML, no exact 44179 production binding exists in `imports/crystalserver/bindings/items.json:1`, and no complete own-ID witness for this item exists in the cited Wiki witness set/reward snapshots. Do not infer a bottle's capacity, weight, fluid subtype or light attributes from its name. Requires a complete qualified donor record and exact binding. |
| 3083 Garlic Necklace | **RESOLVED (Item); Ornamented Shield remains HELD** | `imports/canary/items-xml/items.xml:7272`: **150** charges, weight 380, life-drain protection 20%, necklace slot. Wiki page 2452/revision 1194208 agrees on 150 charges and 3.80 oz. No timer/container. The Ornamented Shield reward's existing `SOURCE_CHARGE_MISMATCH` is preserved by `reward_claim_authoring.py:249`; admitting the Item does not authorize changing the conflicting source reward. |
| 3056 Bronze Amulet | **RESOLVED (Item)** | `imports/canary/items-xml/items.xml:6966`: 200 charges, weight 500, mana-drain protection 20%, necklace slot; no timer/capacity. Wiki page 2466/revision 1194240 agrees. Preserve exact equipment/protection facts; NonStackable, full charges 200, no timer/container. Power Ring's independent `NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED` source-claim qualification remains held (`waiting_item_semantics`); this catalogue repair does not extend the closed source-claim lowering proof. |
| 2881 Cup | **RESOLVED** | `imports/canary/items-xml/items.xml:6150`: pickupable, weight 200. Wiki page 3723/revision 1043630 and client liquidcontainer flag agree. `tools/content-migration/fluid_container_profiles.py:1` already qualifies this ID through Crystal 201189f8 `data/scripts/actions/items/fluids.lua` and durable subtype migration 0560: fluid is KNOWN with fluid_type NOT_APPLICABLE (instance-owned), not an Item child-container capacity. NonStackable, no definition charges/timer/container. Present's fluid instance semantics retain that owner. |
| 2882 Jug | **RESOLVED** | `imports/canary/items-xml/items.xml:6155`: pickupable, weight 750. Wiki page 3726/revision 1042330 and client liquidcontainer flag agree. Same explicit I10a fluid owner as Cup; no guessed fixed liquid, capacity, charge or timer. NonStackable; fluid content remains instance state. |

New Native profiles have only `speed_bonus: 0`, consistent with the established
source importer/Abilities default; no action, charge consumption, collision or
equipment bonus is invented for wolf/collar. All other unknown groups remain
unknown. Descriptions retain the already accepted Wiki punctuation.

## Regeneration and verification

All generated content, registries and pins are produced by repository tooling.
Grizzly's source capture pins the whole shared wolf shard, and the condition-potion
capture pins Scarf's shard, although none of their selected definitions change.
The bounded refresh checks each old descriptor against the exact pre-change Git
baseline and requires every selected row to remain identical before updating it.
The regenerator invokes this proof before the native provider/Prey stages; Grizzly
also exposes `--refresh-canonical-inputs-from` for the same operation. Both normal
digest validators are unchanged. Task Board already pins individual definitions,
so its source packet needs no change.

Initial `python3 tools/content-migration/test_itemhold_q.py` reproduced seven
rejected admissions (4 tests, 7 failing subtests) before the overlay; Cup/Jug were
then added to the resolved cohort after locating their existing fluid owner.
The native regression also failed on the missing native profile lookup (0 passed,
1 failed), proving that it reaches the real artifact/catalogue boundary.

| Command | Result |
| --- | --- |
| `cargo fmt --all` | PASS. |
| `cargo clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings` | PASS, exit 0. |
| `cargo fmt --all --check` | PASS, exit 0. |
| `cargo test --locked -p oteryn-gameplay-content itemhold_q_resolved_catalogue_materializes_and_remaining_holds_reject --quiet` | PASS, 1 regression; verifies compiled/decoded inventory admission for all 9 resolved Items, exact full-charge defaults through the timed adapter, fluid instance ownership and absence of all 3 held native profiles. |
| `python3 tools/content-schema/item-authoring/test_engine_items.py` | PASS, 599 checks and 4 tests (with the prepared environment sourced). The first invocation without that environment stopped for missing `rfc3339-validator`; no package was installed or check disabled. |
| `python3 -m unittest discover -s tools/content-migration -p test_quest_reward_item_semantics.py` | PASS, 28 tests. |
| `python3 -m unittest discover -s tools/content-migration -p test_grizzly_shop_profiles.py` | PASS, 6 tests. |
| `python3 -m unittest discover -s tools/content-migration -p test_condition_potion_profiles.py` | PASS, 6 tests. |
| `python3 -m unittest test_itemhold_q.ItemholdQTests.test_source_overlay_is_idempotent_and_rejects_conflicting_accepted_facts test_itemhold_q.ItemholdQTests.test_shared_shard_repin_rejects_any_changed_shop_definition` (from `tools/content-migration`) | PASS, 2 tests. |
| `cargo test --locked -p oteryn-gameplay-content item_ref::tests --quiet` | PASS, 7 Item catalogue/index tests. |
| `cargo test --locked -p oteryn-game-server --test content_item_identity --quiet` | PASS, 14 canonical identity/held-identity tests. |

The initial `regenerate_content.py --resolve` stopped at
`CONDITION_POTION_CANONICAL_DRIFT`, exposing the second shared-shard pin described
above. No validator was bypassed. The focused catalogue regression then passed
all 6 tests (9 resolved Items, 3 retained holds, exact charge counts, preserved
Ornamented Shield conflict, idempotence/conflict rejection and safe repinning).
`world_project_v2_to_tree.py` then passed. Diff inspection proves exactly 9 changed
canonical Items and exactly 2 added native profiles, with zero removed or changed
existing native profiles. `build_full_spell_artifact.py --refresh-loot-tables`
updated the five existing loot facts that refer to this cohort (admission flags
only; charge counts are preserved).

`regenerate_content.py --resolve --skip-checks` completed successfully, including
all authoring tools and the world compiler's pin refresh. World pin digests were
unchanged. Checks are run separately below rather than omitted.

The dependent Prey simulation rebuilt its receipt (15 cases, 776 creatures).
Semantic comparison confirms that its results, slot values and reports change
only the model-input digest; all numerical results are identical. Taxonomy
changes are source-input digests only (119 receipt leaves), with no category
changes. Generated reward deltas remove the Item hold from Ornamented Shield
while preserving its source charge conflict; Present moves to
`waiting_implementation` with no Item holds; Triangle Tower gains its already
qualified complete native recipe. These are dependency resolutions, not a claim
that every affected quest is runtime-ready.

Final validation covers implementation commit
`0a19c5cba0010ed6c84fa3865459db3218cfcaf9`; the follow-up commit changes only this
report.

| Final command | Result |
| --- | --- |
| `python3 tools/content-migration/regenerate_content.py --resolve --skip-checks` | PASS, exit 0; authoring and pin refresh completed. |
| `python3 tools/content-migration/validate_world_project_v2_to_tree.py` | PASS, exact qualified-tree round trip; 34,111 Items and 336 reward claims. |
| `python3 tools/content-schema/validate_materialized_game_tree.py` | PASS, materialized 97/97, 10 world markers, 1,257 successor files. |
| `python3 tools/content-migration/quest_reward_item_semantics.py --check` | PASS, 525 admissions, zero changed shards. |
| `python3 tools/content-migration/prey_difficulty.py --check` | PASS, 4 derived files checked. |
| `python3 tools/content-migration/test_itemhold_q.py` | PASS, 6 tests after final regeneration. |
| `cargo clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings` | PASS again on the final generated inputs. Only this Rust crate changed. |
| `cargo test --locked -p oteryn-gameplay-content --quiet -- --test-threads=2` | PASS: 312 unit tests, 1 integration test and 6 doc tests; 1 pre-existing ignored test, zero failures (unit suite 656.86 s). |
| `cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet -- --test-threads=2` | PASS, 123 tests, 152 filtered out. |
| `cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet -- --test-threads=2` | PASS, 5 tests (113.95 s). |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/itemhold-q-pin-check` | PASS, exit 0; Oteryn bundle 31,101,773 bytes, digest `3af394d0f4e68a8485d4655e3109ceaf867320c5ca053290a29e8d297faf7afd`. |
| `git diff --check` | PASS. |

The existing ignored test is
`actual_full_manifest_qualifies_source_world_and_all_owner_profiles`, which
requires `OTERYN_FULL_SPELL_TEST_MANIFEST` from the separate explicit source
producer. Its ignore attribute and all existing tests/guards are unchanged.
No full nextest or full PostgreSQL suite was run, as requested.

Self-review covered all handwritten changes, exact canonical/native record
differences, source pin proofs and generated receipt deltas. No unresolved code
finding remains; the three documented Item holds and independent quest source
qualifications remain intentional limits of this repair. No PR was opened.
