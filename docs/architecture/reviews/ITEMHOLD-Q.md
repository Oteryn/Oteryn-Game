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
facts, portability and stack/container/fluid flags. Its original nine canonical
completions are preserved; `itemhold_q_remaining.py` completes the other three.
Existing fluid, timed, starter and bag profiles retain their owners. The single
new Wiki binding has a closed exact-recipe guard; no generic binding predicate or
source charge-disposition rule is relaxed.
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
| 52732 Guardian Ring | **RESOLVED in continuation** | Retained Crystal `ff7ede593c69d4c658b382c97443e8155926924a`, `imports/ots-source-evidence/item-fx-audio295/retained-source-files.json:477` (`source_files[47].raw_utf8`, decoded `data/items/items.xml:87588`): weight 90, armor 1, ring moveevent, no charges/timer/capacity. Wiki `imports/tibiawiki/facts/items-stats.json:231916`, page 109358/revision 1194069, agrees on 0.90 oz, armor 1, Finger and level 0. Existing exact Crystal identity is reused; NonStackable, no timer/charges/container, unrestricted level-0 ring pattern. |
| 3054 Silver Amulet | **RESOLVED** | `imports/canary/items-xml/items.xml:6946`: 200 charges, weight 500, poison/earth protection 10%, necklace slot; no decay/duration/capacity. Reward admission observation page 2829/revision 1194271 agrees on charges, weight and Neck. Preserve those exact existing equipment/protection facts; NonStackable, no timer/container, full charges 200. Existing timed host adapter `crates/gameplay/content/src/content/timed_item_definition.rs:57` resolves definition charges; no source tuple is reinterpreted as quantity. |
| 14156 Dog Collar | **RESOLVED** | `imports/canary/items-xml/items.xml:32492`: Noodles description and weight 110; no charges/duration/capacity. Wiki witness `57262:0`: exact item ID and weight 1.10 oz. Pinned client has take=true, no cumulative/container/unmove flag. NonStackable, no timer/charges/container; preserve accepted description. Add exact native profile using existing Crystal binding. |
| 12369 Captured Wolf | **RESOLVED** | `imports/canary/items-xml/items.xml:29995`: captured-wolf description, weight 5000; no charges/duration/capacity. Wiki witness `54281:0`: 50.00 oz and stackable no. Client proves portability/nonstackability. NonStackable, no timer/charges/container; preserve accepted description. Add exact native profile using existing Crystal binding. |
| 33330 Monolith Kit | **RESOLVED in continuation** | Canary `imports/canary/items-xml/items.xml:62867` and retained Crystal XML say 750 centi-oz (7.50 oz). Wiki `imports/tibiawiki/facts/items-stats.json:151374`, page 92100/revision 1013917, hash `dc578d743ac45e14f7e8bd11c9bf1fd8c325f00349da29a47075d31ce20f940b`, says 75.00 oz. ITEM-SEM source precedence selects **7500**; the conflicting donor value remains a checked witness. Client proves portable/nonstackable/non-container and height=true. Existing exact Crystal identity reused; no charges/timer/container. |
| 39546 Primal Bag | **RESOLVED** | `imports/canary/items-xml/items.xml:73191`: weight 16000, no capacity/charges/timer. Wiki witness `98740:0`: 160.00 oz, stackable no. Client container=false: this is an opening reward token, not a container with guessed capacity. Preserve the existing reward-bag owner (`tools/content-migration/reward_bag_profiles.py:1`, retained `tools/content-schema/native-gameplay/reward-bag-profiles.json:1`). NonStackable, no container/charges/timer. |
| 44179 Bottled Dragon Breath | **RESOLVED in continuation** | Wiki `imports/tibiawiki/facts/items-stats.json:199056`, page 103946/revision 1041565, hash `eb5189ea4ef782741a3b67bd2794c5e5e72f2f6ed7bc86ad553cba1ccd9ac929`, supplies **0.60 oz = 60**. Own itemid 44179 is retained at `imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json:440736`. Exact client object 44179 proves name, take, nonstackability, non-container and light brightness 2/color 186. No Crystal binding is fabricated: a closed Wiki itemid/revision binding is admitted only for this exact canonical identity and exact 60-weight recipe. Constructor defaults cited below supply no charges/timer; light remains the existing client projection. |
| 3083 Garlic Necklace | **RESOLVED (Item and source projection)** | Existing Item admission remains unchanged: Canary XML:7272 and Wiki page 2452/revision 1194208 give 150 charges, weight 380 and life-drain +20%. The continuation qualifies one full-definition-charge reward for Ornamented Shield only; raw donor charge argument 1 and its source conflict remain in `source_variant`, while the canonical native container projection uses the stronger Wiki/default interpretation. |
| 3056 Bronze Amulet | **RESOLVED (Item and source projection)** | Existing Item admission remains unchanged: Canary XML:6966 and Wiki page 2466/revision 1194240 give 200 charges, weight 500 and mana-drain +20%. Power Ring UID6152 now qualifies one full-definition-charge instance; exact raw 200 equals the admitted definition. No partial-charge persistence is invented. |
| 2881 Cup | **RESOLVED** | `imports/canary/items-xml/items.xml:6150`: pickupable, weight 200. Wiki page 3723/revision 1043630 and client liquidcontainer flag agree. `tools/content-migration/fluid_container_profiles.py:1` already qualifies this ID through Crystal 201189f8 `data/scripts/actions/items/fluids.lua` and durable subtype migration 0560: fluid is KNOWN with fluid_type NOT_APPLICABLE (instance-owned), not an Item child-container capacity. NonStackable, no definition charges/timer/container. Present's fluid instance semantics retain that owner. |
| 2882 Jug | **RESOLVED** | `imports/canary/items-xml/items.xml:6155`: pickupable, weight 750. Wiki page 3726/revision 1042330 and client liquidcontainer flag agree. Same explicit I10a fluid owner as Cup; no guessed fixed liquid, capacity, charge or timer. NonStackable; fluid content remains instance state. |

New Native profiles have only `speed_bonus: 0`, consistent with the established
source importer/Abilities default; no action, charge consumption, collision or
equipment bonus is invented for wolf/collar. All other unknown groups remain
unknown. Descriptions retain the already accepted Wiki punctuation.

## Initial delivery: regeneration and verification

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
finding remained in the initial delivery. Its three Item holds and two source qualifications
are superseded by the continuation below. No PR was opened.

## Continuation from 1516faabd: no remaining ITEMHOLD-Q holds

The owner continuation of 2026-11-06 requires applying documented precedence rather
than deferring conflicts. The nine resolved records from `1516faabd` are not redone.
`itemhold_q_remaining.py` adds only 52732, 33330 and 44179 and three native profiles.

The atomic-field policy is `tools/content-schema/item-authoring/lower_wiki_stats_packet.py:4`:
TibiaWiki supplies selected values over Crystal/Canary hypotheses. The general ordering
is `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md:455`.
For Monolith this selects Wiki 7500 over donor 750, while checking and retaining both.
For the bottle, Wiki 60 fills UNKNOWN; it does not guess weight from the name. All
three own-ID/page/revision/content-hash witnesses and the client hash are checked.

The retained Crystal constructor is
`imports/ots-source-evidence/item-fx-audio295/retained-source-files.json:17`,
`source_files[1].raw_utf8` (`src/items/items.hpp`, SHA-256
`4bb45e54fc06921e82b12badee389330376f72f352b2ef8c0074e824cf7b9844`):
line 38 initializes ability speed to 0, line 284 decayTime to 0, line 288 charges
to 0. The bottle has no XML override; the admitted appearance says non-container,
non-fluid, non-clothing and portable. These loader defaults supply the bounded native
recipe; unspecified use effects remain unknown. Native collision flags are source-proven
false for all three; height is true only for Monolith. Bottle light 2/186 remains the
already admitted 15.30 appearance projection, not a new invented server attribute.

Bottled Dragon Breath reuses canonical `oteryn:item.tibia.i44179/definition-r1` with
`oteryn:source.tibiawiki`, namespace `tibiawiki/item_id`, external ID 44179 and revision
`fandom-r1041565`. The native decoder adds that **single exact binding**, rejects a
collision with any donor binding, checks exact canonical semantics plus weight 60,
portability, nonstackability, no charges/timer/container/fluid, legal destinations and
exact source-qualified attributes. Every existing binding/semantic validator remains
active. Tests reject a stale Wiki revision and a forged weight; unrelated IDs remain
unadmitted. No Crystal XML row or identity allocation is fabricated.

### The two source-claim qualifications

`itemhold_charge_precedence.py` is closed to the exact keys, positions, IDs and raw
arguments below. Both cases require the exact current Item charge witness and both
English/BR quest reward-entity witnesses. Physical quantity one is retained from the
existing charge-subtype normalization; it does not come from an omitted Wiki quantity.
The owner continuation selects the full current-definition-charge native interpretation
for these two cases. Original OTS arguments and source conflicts remain immutable
historical witnesses; they no longer block the separately qualified native projection.

| Claim | Retained donor | Selected stronger witnesses / native result |
|---|---|---|
| Ornamented Shield `kv/ornamentedshield`, (32769,32302,10), UID6063 | `tools/content-schema/reward-claim-authoring/container_native_source.json:1`, entry line 1051 of Crystal `data-global/startup/tables/chest.lua`: `[3083,1]`; preserve the mismatch against 150 | English quest page 3778/revision 1043470 and BR page 4489/revision 378590 agree on Garlic Necklace (`wiki_quest_facts.json:41180`, `:144198`); Item page 2452/revision 1194208 proves 150. One full 150-charge necklace reaches the native container reader. |
| Power Ring `kv/powerring2`, (32601,31776,9), UID6152 | `source_charge_evidence.json:209`: raw/default 200; Canary chest.lua:1477, Crystal chest.lua:1762 | English page 3282/revision 683946 and BR page 4496/revision 201567 agree on Bronze Amulet (`wiki_quest_facts.json:36995`, `:144905`); Item page 2466/revision 1194240 proves 200. One full 200-charge instance reaches the native leaf reader. |

This extends the closed full-charge proof, not the generic partial-charge rule.
Other placements, raw values, identities, source conflicts and unsupported variants
still reject. Absent timed state resolves through the existing full-definition timed
adapter; no new durable schema or partial-state writer is needed. Canonical variant
`waiting_implementation`/source dispositions are historical vocabulary: the exact
`native_contents` projection is the existing executable W15b admission boundary.
Power Ring's plain claim becomes `ready`. Native recipe counts are 69 (68 at 1516faabd,
adding Ornamented Shield), ready plain claims 221 (adding Power Ring); strict count
assertions were updated to these exact outcomes, not removed.

### Continuation validation

The expanded Python test reproduced the three held Item rejections and the blocked
Ornamented projection on 1516faabd. After generation, all 9 focused Python tests pass;
the compiled/decoded native regression passes for all 12 Items, exact weights and charge
defaults, and rejects forged bottle weight/revision. The constructor snapshot is also
digest-bound and a mutation test rejects changed loader defaults.

Semantic scope comparison against `1516faabd` proves exactly three changed canonical
records (52732, 33330, 44179), with all prior nine and all existing native profiles
unchanged. Four native profiles are added: the three remaining Items, plus the already
canonical Lyre 2949 needed to close Ornamented Shield's complete five-child reward.
Lyre's accepted weight is 1250 and the donor is
`imports/canary/items-xml/items.xml:6397`; the existing quest-callback generator supplies
that profile. No new canonical Lyre admission or use effect is introduced.

| Final continuation command | Result |
|---|---|
| `python tools/content-migration/regenerate_content.py --resolve --skip-checks` | PASS; complete regeneration and compiler pin refresh; checks below run separately |
| `python tools/content-migration/validate_world_project_v2_to_tree.py` | PASS after final constructor guard; exact qualified round trip, 34111 Items / 336 claims |
| `python tools/content-schema/validate_materialized_game_tree.py` | PASS; 97/97, 10 markers, 1257 successor files |
| `python tools/content-migration/quest_reward_item_semantics.py --check` | PASS; 525 admissions, zero changed shards |
| `python tools/content-migration/test_itemhold_q.py` | PASS; 9 tests, all 12 Items, strict constructor pin and closed source projections |
| `cargo test --locked -p oteryn-gameplay-content itemhold_q --quiet` | PASS; 2 compiled native catalogue/claim-reader regressions, including forged weight/revision rejection |
| In `tools/content-schema/reward-claim-authoring`: `python -m unittest test_reward_claim_variant_authoring test_reward_claim_variant_migration test_container_native` | PASS; 42 tests after correcting a root-parameter regression; immutable raw conflicts and all other blocked shapes remain checked |
| `cargo test --locked -p oteryn-gameplay-content --quiet` | PASS; 313 unit + 1 integration + 6 doc tests; zero failures, one pre-existing ignored unit test |
| `python tools/content-schema/item-authoring/test_engine_items.py` | PASS; 599 checks and 4 tests |
| `cargo clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings` | PASS; only changed Rust crate |
| `cargo fmt --all --check` | PASS |
| `cargo test --locked -p oteryn-game-server --test content_item_identity --test content_world_project_repository --quiet -- --test-threads=2` | PASS; 14 identity + 5 repository-content tests |
| `cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet -- --test-threads=2` | PASS; 123 tests, 152 filtered out |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/itemhold-q-cont-pin-check` | PASS; 31101773 bytes, digest `3af394d0f4e68a8485d4655e3109ceaf867320c5ca053290a29e8d297faf7afd` |
| SHA-256 comparison of all tested Item shards/providers, Creature provider, loot facts and reward shards before/after regeneration | PASS; byte-identical, so the completed content suite covers the final generated inputs |
| `git diff --check` | PASS |

All remaining ITEMHOLD-Q catalogue and selected source-claim holds are resolved.
The initial candidate was preserved locally before fast-forwarding to `1516faabd`;
none of its duplicated nine-item work was reapplied. No full nextest or full PG suite
was run, no protected branch was modified, and no PR was opened.

## PR #3017 merge repair (2026-11-06)

Merged main `ffef24ad6` into `16aa17f19` with a merge commit, preserving
main's starter admissions, cosmetics and small Item update provider, and the
ITEMHOLD-Q admissions and exact source recipes. No source admission was dropped.
The rejection regression now uses held Item 1000 (dry grass, unknown semantics)
and unknown Item 54609: main now legitimately admits starter Item 34017.

Conflicts: `content/creatures/prey-slot-values.json`,
`content/spells.manifest.json`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`,
`docs/balance/PREY-TIME-SIM-2026-11-01.md`,
`tools/content-schema/native-gameplay/loot-tables.json`, and
`tools/content-schema/native-gameplay/prey-model-results.json`. The registered
regeneration driver also handled `content/items/index.json`. The loot table
was rebuilt with `build_full_spell_artifact.py --refresh-loot-tables`; all other
derived conflicts and bundle pins use `regenerate_content.py --resolve
--skip-checks`, followed by the checks recorded below. No generated output or pin
was edited by hand.

An early content test run was stopped after it exposed the expected stale
pre-merge Item update base pin. Only the post-regeneration results below count
as final validation.

The generator completed its catalogue steps; its final `pin-refresh` process
was killed by the memory limit while an early pin-check was running concurrently.
The duplicate check was stopped. The final two bundle commands below ran
sequentially and passed; no generator or validation rule was weakened.

| Merge validation command | Result |
| --- | --- |
| `python tools/content-schema/native-gameplay/build_full_spell_artifact.py --refresh-loot-tables` | PASS; rebuilt the merged loot admission facts |
| `python tools/content-migration/regenerate_content.py --resolve --skip-checks` | Catalogue outputs regenerated and conflicts staged; final pin refresh resumed separately after the memory kill described above |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-refresh .` | PASS; reproduced main's bundle digest `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671` |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/itemhold-q-merge-pin-check-final` | PASS; 32,839,269 bytes, same digest |
| `python tools/content-migration/validate_world_project_v2_to_tree.py` | PASS; strict source recomputation, 34,111 Items and 476 Item bindings |
| `python tools/content-schema/validate_materialized_game_tree.py` | PASS; 97/97 materialized, 10 world markers, 1,257 successor files |
| `python tools/content-migration/quest_reward_item_semantics.py --check` | PASS; 525 admissions, no changed shards |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings` | PASS after regeneration |
| `python tools/content-migration/test_itemhold_q.py` | PASS; 9 tests after regeneration |
| `python tools/content-schema/item-authoring/test_engine_items.py` | PASS; 599 checks and 3 + 4 unit tests |
| `python -m unittest test_reward_claim_variant_authoring test_reward_claim_variant_migration test_container_native` (reward-claim-authoring directory) | PASS; 42 tests |
| `cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet -- --test-threads=2` | PASS; 123 tests after regeneration |
| SHA-256 comparison of all 529 content JSON inputs before and after the final pin refresh | PASS; unchanged |

The canonical Item shards, native base provider and reward-claim shards were
also byte-identical before and after regeneration, so the Item engine and
source-claim checks above cover the final generated content. Only the generated
SRX-1 base pin and Prey value table changed in those initial tested inputs; the
final content suite and server tests use the refreshed files.

| Additional first-merge checks | Result |
| --- | --- |
| `cargo test --locked -p oteryn-gameplay-content --quiet` (first merge) | PASS; 315 unit + 1 integration + 6 documentation tests, one pre-existing ignored test |
| `cargo test --locked -p oteryn-game-server --test content_item_identity --test content_world_project_repository --quiet -- --test-threads=2` (first merge) | PASS after regeneration; 14 + 5 tests |

### Final remote refresh

The requested final fetch found main advanced to `91afbe0ba` by the SRV11
cast/spectate change (#3016), while the task branch remained `16aa17f19`.
The validated `ffef24ad6` repair was committed as merge `989ac9ca6`; the newer
main was then merged without conflicts. Neither merge rebases or discards either
parent. Regeneration is repeated on the combined branch. The commit author and
committer are `blakinio <75369544+blakinio@users.noreply.github.com>`, and GitHub
authentication is `blakinio`.

Latest-main checks use the same commands in the merge-validation table above:

| Check on combined `91afbe0ba` branch | Result |
| --- | --- |
| Full `oteryn-gameplay-content` suite, including ITEMHOLD-Q and Item reference regressions | PASS; 315 unit + 1 integration + 6 documentation tests; one existing ignored test |
| Content Item identity / world repository server tests | PASS; 14 + 5 tests |
| REST `content_` server tests | PASS; 123 tests |
| Strict source recomputation | PASS; 34,111 Items, 476 bindings |
| Materialized-tree validator | PASS; 97/97 |
| Quest reward Item semantics `--check` | PASS; 525 admissions, unchanged shards |
| `cargo fmt --all --check` | PASS |
| Content crate clippy, all targets, `-D warnings` | PASS |
| Python ITEMHOLD-Q catalogue tests | PASS; 9 tests |
| Item engine suite | PASS; 599 checks and 3 + 4 unit tests |
| `python tools/content-migration/regenerate_content.py --resolve --skip-checks` on latest main | PASS, including the final sequential bundle pin refresh |
| Whole `content/` tree compared with first merge `989ac9ca6`, plus all 529 tested JSON SHA-256 hashes | PASS; byte-identical after final regeneration, so all completed checks cover the final content |
| `git diff --check`, staged diff check and unresolved-conflict scan | PASS |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/itemhold-q-latest-main-pin-check` | PASS; 32,839,269 bytes, digest `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671` |

No full nextest or full PG suite was run. No PR was opened, no protected branch
was modified, and neither merge used rebase or force push.
