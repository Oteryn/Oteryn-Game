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

## CI repair after merge head b0d3b5b92

The reward migration packet bound older Item/claim inputs. It was regenerated
with the documented `reward_claim_variant_migration.py` write mode (no `--check`),
then its byte-for-byte check passed. The quest checks failure was the obsolete
38-blocked-variant assertion: Ornamented Shield and Triangle Tower now qualify,
leaving exactly 36. The test now also removes only those two projections and
proves that the original 38 failures return, with exactly those two identities.

The quest unit job additionally exposed Grizzly room provider-pin drift, old
completion-matrix totals, and four historical SOURCE claim views following live
admissions. The SOURCE compatibility projection preserves all 284 frozen SOURCE
cores, the 242 immutable chosen recipes/selection/baseline, and the existing 28
approved core pairs. The narrowly scoped receipt generator verifies every other
approval input and refreshes only the SOURCE compiler pin and its approval hash.
Live canonical claims retain their resolved holds/native contents. New regressions
reject SOURCE core changes and any other approved-field change.

The Quest tree, binding packets, completion/rollout matrices, v2 export, state
lowering, completion import, source/collect/use/completion binding candidates and
Grizzly room packet were rebuilt through their owning generators, in the
dependency order documented in the Quest authoring README. No generated packet,
receipt, content shard or pin was edited by hand.

| CI-equivalent command | Result |
| --- | --- |
| `python tools/content-schema/quest-authoring/run_checks.py --part checks` | PASS |
| `python tools/content-schema/quest-authoring/run_checks.py --part unit-tests` | PASS; 93 modules, 1,052 tests; pre-existing skips retained |
| `python tools/content-schema/quest-authoring/run_checks.py --part donor` | PASS |
| `python tools/content-schema/reward-claim-authoring/test_reward_claim_variant_migration.py` | PASS; 17 tests |
| `python tools/content-schema/reward-claim-authoring/test_reward_claim_authoring.py` | PASS |
| `python tools/content-migration/validate_world_project_v2_to_tree.py` | PASS |
| `amend_itemhold_source_receipt.py --check` and `grizzly_rooms.py --check` | PASS |

All 69 generator-check invocations discovered in the item/reward content
workflows and the Quest check driver passed. The source-backed checks use the
workflows' exact pinned upstream revisions and verify their digests. The full
check inventory follows (commands run from the repository unless shown below):

| Generator check | Result |
| --- | --- |
| `python tools/content-schema/item-authoring/lower_client_physical_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_timed_item_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_equip_abilities_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_ranged_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_use_observation_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_forge289_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/item_weapon_proficiency.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_client_market_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_wiki_movable_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_client_market_d289_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_wiki_movable_d289_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_document_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_description_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_description_wiki_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_name_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_name15_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_item_hit_magic_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py --check` | PASS |
| `python tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py --check` | PASS |
| `python tools/content-schema/reward-claim-authoring/reward_claim_authoring.py content --check` | PASS |
| `python tools/content-schema/reward-claim-authoring/container_source.py --check --verify-upstream` | PASS |
| `python tools/content-schema/craft-authoring/craft_authoring.py content --check` | PASS |
| `python tools/content-schema/imbuement-authoring/imbuement_content.py content --check` | PASS |
| `python tools/content-schema/starter-kit-authoring/starter_kit_authoring.py content --check --seal-base origin/main` | PASS |
| `python tools/content-schema/proficiency-authoring/proficiency_authoring.py build --check` | PASS |
| `python tools/content-schema/achievement-authoring/build_catalogue.py --check` | PASS |
| `python tools/content-schema/wheel-authoring/build_schema.py --check` | PASS |
| `python tools/content-schema/wheel-authoring/wheel_authoring.py build --check` | PASS |
| `python tools/content-schema/wheel-authoring/verify_item_assets.py --check` | PASS |
| `python tools/content-schema/wheel-authoring/client_icons.py --check` | PASS |
| `python tools/content-schema/wheel-authoring/import_server_data.py --check` | PASS |
| `python tools/content-schema/wheel-authoring/build_report.py --check` | PASS |
| `python tools/content-schema/item-authoring/client_appearance_census.py --check --crystal-source /tmp/itemhold-ci-sources/crystal-base --donor-source /tmp/itemhold-ci-sources/crystal --canary-source /tmp/itemhold-ci-sources/canary-base` | PASS |
| `python tools/content-schema/item-authoring/appearance_membership.py --check --source crystal-ff7ede5=/tmp/itemhold-ci-sources/crystal-base/data/items/appearances.dat --source canary-47dfd51f=/tmp/itemhold-ci-sources/canary-base/data/items/appearances.dat --source crystal-donor-00ce02a5=/tmp/itemhold-ci-sources/crystal/data/items/appearances.dat` | PASS |
| `python tools/content-schema/world-object-authoring/build_catalogue.py --source /tmp/itemhold-ci-sources/crystal-base --donor-source /tmp/itemhold-ci-sources/crystal --official-client --qualified-world --check` | PASS |
| `python imports/ots-source-evidence/item-fx-audio295/producer/raw_fx_audio_reimport.py --capture /workspace/Oteryn-Game/imports/ots-source-evidence/item-fx-audio295/raw-source-staging.json --sources /workspace/Oteryn-Game/imports/ots-source-evidence/item-fx-audio295/retained-source-files.json --batch /workspace/Oteryn-Game/imports/ots-source-evidence/item-fx-audio295/import-batch.json --check` | PASS |
| `python tools/content-schema/native-gameplay/build_presentation_bindings.py --check --reference-engine /tmp/itemhold-ci-sources/canary-cues --crystal-engine /tmp/itemhold-ci-sources/crystal` | PASS |
| `python tools/content-schema/native-gameplay/build_creature_races.py --check --canary /tmp/itemhold-ci-sources/canary-monsters --crystal /tmp/itemhold-ci-sources/crystal` | PASS |
| (Quest authoring directory) `python soul_war_reconstruction.py --check` | PASS |
| (Quest authoring directory) `python ../encounter-authoring/soul_war_crystal_reconstruction.py --check` | PASS |
| (Quest authoring directory) `python ../reward-claim-authoring/reward_claim_authoring.py content --check` | PASS |
| (Quest authoring directory) `python ../reward-claim-authoring/reward_claim_variant_migration.py --check` | PASS |
| (Quest authoring directory) `python refresh_quest_source_checks.py --check` | PASS |
| (Quest authoring directory) `python quest_catalogue_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_completion_matrix.py --check` | PASS |
| (Quest authoring directory) `python collect_claim_candidates.py --check` | PASS |
| (Quest authoring directory) `python use_trigger_candidates.py --check` | PASS |
| (Quest authoring directory) `python source_text_authoring.py --check` | PASS |
| (Quest authoring directory) `python build_wiki_source_schema.py --check` | PASS |
| (Quest authoring directory) `python wiki_source_inventory.py --check` | PASS |
| (Quest authoring directory) `python wiki_source_supplements.py --check` | PASS |
| (Quest authoring directory) `python wiki_all_source_inventory.py --check` | PASS |
| (Quest authoring directory) `python wiki_all_source_inventory.py --schema-out wiki_all_source_specs.schema.json --check` | PASS |
| (Quest authoring directory) `python wiki_requirement_interpretations.py --check` | PASS |
| (Quest authoring directory) `python source_facts_authoring.py --specifications samples/wiki-source-all373/source-specs-373.json --samples samples/unbound-source-facts --check` | PASS |
| (Quest authoring directory) `python authored_quest_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_completion_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_rollout_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_binding_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_v2_export.py --check` | PASS |
| (Quest authoring directory) `python quest_state_lowering.py --check` | PASS |
| (Quest authoring directory) `python source_lowered_binding_plan.py --check` | PASS |
| (Quest authoring directory) `python quest_completion_import.py --check` | PASS |
| (Quest authoring directory) `python quest_native_binding_preflight.py --check` | PASS |
| (Quest authoring directory) `python quest_reward_only_binding_candidates.py --check` | PASS |
| (Quest authoring directory) `python bundle_authoring.py --check` | PASS |
| (Quest authoring directory) `python quest_tree_authoring.py content --check --source-packet /workspace/Oteryn-Game/tools/content-schema/quest-authoring/samples/migration/quest-source-packet.json` | PASS |

### Rust failure reproductions and scoped corrections

On the unchanged `b0d3b5b92` checkout, nextest reproduced
`equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes`
with 3,610 actual profiles versus the old 3,604 expectation. The exact six-key
admission delta is 2949, 12369, 14156, 33330, 44179 and 52732; existing profiles'
speed attributes are unchanged. The independent cohort inventory now records
those six additions, and the regression explicitly requires each to be
materializable with source-qualified zero speed. Existing unknown-speed and
ability checks remain intact. The corrected test passed in the full dispatch run.

The unchanged native-writing test belongs to nextest hash shard 1/3. Stress
iteration 4/100 reproduced its `inventory_select(item.handle)` assertion failure.
The exact six-line fix from `726eaa0f7` on the cached `-gpkg` branch waits for
initial inventory publication after mail readiness, bounded by three seconds;
all selection, UTF-8, saving and write-once refusal assertions are preserved.
After porting it, all 100 stress iterations passed.

| Additional command | Result |
| --- | --- |
| `cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings` | PASS |
| `cargo clippy --locked -p oteryn-client --all-targets --quiet -- -D warnings` | PASS |
| `cargo nextest run --locked -p oteryn-client --lib -E 'test(srx1_native_writing_selected_item_utf8_save_and_write_once_refusal)' --stress-count 100` | PASS; 100/100 after the port; original fails at iteration 4 |
| `cargo fmt --all --check` after the dispatch correction | PASS |
| `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/itemhold-repair-final-pin` | PASS; bundle `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671` |
| S17 cast/viewer regression selection from the compiled dispatch binary | PASS; 9 tests, including real PostgreSQL persistent bans and viewer projection |

The local PostgreSQL server was stopped when the first baseline SQL test ran.
It was restarted using the prepared PostgreSQL 18.6 data directory; the same SQL
reads/share-lock test then passed without a code change. This environment error
is separate from the reproduced profile-count and native-writing regressions.

### Shared PostgreSQL concurrency correction after the CI report

The five failed jobs in run `42292351375` are partitions 8, 15, 16 and 21
(library step), and 22 (registered E2E step). GitHub's job metadata confirms
those steps; downloading the exception logs from its storage host returns
`Tunnel connection failed: 403 Forbidden`. Local reproduction is used below;
a partition-to-exception attribution is not inferred from a matching label.

Ported the four common-file changes from `-gpkg` commit
`886b1c77622582fdb94c66940e271aada9c6e751`, preserving this branch's other work.
Inspection now holds the spell-lane permit across each complete Item, Trade or
Player read, with Trade passing that permit into its helper. The history-gap
probe takes admission guard locks before the reconnect-session row, matching
live admission's lock order. The regression runs a peer staff refresh alongside
Player inspection and retains all successful-result assertions.

Applying only that concurrent regression to the old production code reproduced
`inspection_postgres_trade_counterpart_requires_current_offer_and_leaf_index`:
`player_trade.rs:359`, actual result 2 versus required 1 (184.164 seconds).
This demonstrates the inspection race independently of the inaccessible CI log.
The initially running dispatch diagnostic was superseded before the production
patch; its partial success is not reported as a completed final suite.

The separate complete PostgreSQL-enabled durability/server library diagnostic
passed **1,253/1,253** tests (three pre-existing skipped tests):
`cargo nextest run --locked -p oteryn-gameplay-durability -p oteryn-game-server --lib --no-fail-fast`.

The registered E2E listing was rebuilt with all four workflow targets (858 cases).
Applying the workflow's unchanged 24-way duration planner selects 41 cases for
partition 22. Its exact nextest filter union passed **41/41** with PostgreSQL
18.6 enabled, including `s17_cast_postgres_cross_worker_consent_barrier`.
The original partition-22 exception has not reproduced locally; no test or
runtime constraint was relaxed to obtain this result.

Replaying the actual E2E listing before assigning the library cases puts the
boss readiness-history probe in partition 8, concurrent Trade/Player inspection
in 15, and the production speed-profile inventory in 16. These match three red
partitions and the independently established defects. Partition 21 was subsequently reproduced as the stale callback inventory
(documented below). Both updated Rust crates' Clippy runs pass with `-D warnings`, and
`cargo fmt --all --check` passes on the combined patch.

The first combined library-list build was killed with `signal: 9, SIGKILL`;
the container's memory events recorded an OOM kill. It is retried with
`CARGO_BUILD_JOBS=1`. A repeated bundle pin-check used approximately 8 GB and
was stopped while compilation was active; the earlier completed pin-check
already covers the unchanged generated content. Compiler caches and logs are
outside the repository, and no source or test assertion was changed for this
environment failure.

The serialized retry succeeded (8m19s). The fresh nextest library listing
excludes the 21 pre-existing ignored cases before the workflow planner runs;
using it gives 158/163/163/157 runnable cases for partitions 8/15/16/21,
respectively (641 total). The three identified regressions remain assigned to
8/15/16. These exact filter unions run with the workflow's duration priorities.

The complete client library suite also passed **342/342**, zero skipped:
`cargo nextest run --locked -p oteryn-client --lib --no-fail-fast --test-threads 1`.
The native-writing stress regression additionally passed 100/100 as recorded
above. Final dispatch Clippy passed after porting the shared PostgreSQL patch.

### Additional failures found by the complete library runs

The first full dispatch run completed in 2355.939 seconds: 2,572 passed,
one failed, and 18 pre-existing ignored tests. It reproduced
`quest_callbacks_binding_and_coverage_require_exact_source_witnesses`:
`docs/repository/QUEST_CALLBACK_COVERAGE.json` listed 308 supported claims,
but the real binder listed 311. The exact difference is `kv.powerring2`,
`kv.triangletower` and `kv.ornamentedshield`. The actual workflow planner puts
this case in failed PG partition 21, so its cause is this packet's claim admissions.

Regenerated that inventory with its owner, without modifying the Rust assertion:
`python tools/content-schema/quest-authoring/quest_callback_coverage.py --canary /tmp/itemhold-callback-donors/canary --crystal /tmp/itemhold-callback-donors/crystal`.
The matching `--check` passes. Retained donor blobs are Canary `47dfd51f`,
`data-otservbr-global/startup/tables/chest.lua`, SHA-256
`2b130d4196afc9700eb0b7c2f5594e331edee4d25811a0d2020fb9d8757d6cda`,
and Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/startup/tables/chest.lua`, SHA-256
`7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4`.
Both hashes are checked by the existing generator. This source-bound inventory
check supplements the 69 workflow generator checks recorded above.

The PostgreSQL run additionally reproduced partition 8's
`charm_wire_missing_snapshot_preserves_combat_and_assignment_changes_damage_and_dodge`:
all four assignments executed, but the strict Dodge comparison failed at
`kill_reward_postgres_e2e_tests.rs:101`. The original run used real elapsed time
plus manual clock shifts, so the compared scenarios did not supply identical
semantic times. A focused experiment keeps all original assertions and 64
samples while feeding the same explicit times through the existing monster
clock seam and an equivalent test-only auto-attack entry point. The production
auto-attack entry point still samples its clock at the original location.


### Final focused CI repair (owner directive: 2026-11-06 23:55 UTC)

The owner requested stopping broad runs and publishing validated fixes within
30 minutes. Broad runners were stopped; their partial results are diagnostic,
not completed green suites. The completed client suite (342), quest unit job
(1,052), durability/server PG libraries (1,253), and partition-22 E2E share (41)
remain completed passing runs. Subsequent verification is limited to failing
regressions and changed-crate fmt/Clippy.

The synchronized 64-turn charm experiment still failed, with identical plain
and Dodge damage vectors. A temporary single-scenario trace then proved that
all **31** actual Cave Rat incoming hooks reached stage-3 Dodge and returned
`NotTriggered`. Clock drift was therefore not the cause of the missing proc:
the changed content decision root produces no proc in that bounded window.
The final fixture uses 256 identical semantic turns per assignment, retains
all existing positive-effect and pointwise damage assertions, and additionally
requires equal vector lengths. Neither the 11% source chance nor production
combat/draw rules change. The explicit test-time seams keep the counterfactual
comparisons on matching turns; production still reads its clock at the original
locations. Temporary trace code and its diagnostic test were removed.

Partition 16 additionally reproduced
`reaudit_relocation_tiles_monster_push_trap_entry_exit_postgres`: 50 HP became
18, outside its stale raw 15..30 expectation. Trap 3944 is Earth damage
(`paced_movement/special_movement.rs:63`); canonical Troll has mitigation 8/25
percent and Earth resistance -10 percent
(`content/creatures/definitions/spell-native-profiles.json:1`). The existing
source condition finisher truncates mitigation then rounds resistance
(`crates/gameplay/entities/src/carrier/runtime_actor_periodic.rs:620`). For raw
15..30 this gives exactly `{15,17,18,19,20,21,22,23,24,25,26,28,29,30,31,32}`.
The regression now asserts those pinned policy values and this exact one-hit
set; all trap transformation, refusal, reopening and wolf-capture assertions
remain intact. Its focused PostgreSQL rerun **PASSed** (89.590 s).

Focused proven red/green corrections also include:
- production profile inventory: 3,604 expected versus 3,610 actual; six exact
  admitted keys now checked independently; corrected regression passes;
- callback coverage: 308 manifest versus 311 real binder claims; regenerated
  pinned-donor inventory, matching `--check`, and exact Rust regression pass;
- concurrent inspection: result 2 versus required 1 before the shared `886b1c776`
  fix; strengthened concurrent regression passes with PostgreSQL (125.527 s);
- native writing: original fails stress iteration 4; exact `726eaa0f7` startup
  publication fix passes 100/100 stress iterations and the complete client suite.

Final changed-crate checks:
- `cargo fmt --all --check`: PASS.
- `CARGO_BUILD_JOBS=1 cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-client --all-targets --quiet -- -D warnings`: PASS.
- RewardClaim variant migration `--check`: PASS again after the final packet edits.
- Pinned-donor quest callback coverage `--check`: PASS again.
- The completed bundle pin-check covers the unchanged generated content,
  digest `6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671`.


Final focused Dodge result: **PASS**, 389.689 seconds, all four real PostgreSQL
assignments completed. Command:
`CARGO_BUILD_JOBS=1 cargo nextest run --locked -p oteryn-gameplay-dispatch --lib -E 'test(charm_wire_missing_snapshot_preserves_combat_and_assignment_changes_damage_and_dodge)' --no-capture`.
The exact same strict Dodge comparison failed before the fix (802.196 s), and
again with synchronized clocks but only 64 turns (1033.137 s); the hook trace
identified 31 genuine, untriggered draws. Expanding the bounded observation
window to 256 is the validated correction, with matched semantic clocks and
all positive-effect/pointwise assertions retained. No production chance,
validator, deadline or materialization rule was weakened.

At the owner's deadline-driven handoff, all locally reproduced failures are
fixed and their focused regressions are green. Broad diagnostic suites were
stopped as requested and are not represented as completed passing suites.
The original partition-22 exception remains unavailable because its log download
returns `Tunnel connection failed: 403 Forbidden`; its exact 41-case E2E share
passed locally without changing those tests. GitHub CI on the pushed commit is
the publication check; no PR was opened by this task.

### Qualification ledger follow-up on ddc18edf8

`python tools/content-schema/quest-authoring/test_quest_claim_qualification.py`
reproduced the CI failure: the old 297-entry ledger omitted Ornamented Shield,
Power Ring 2 and Triangle Tower after their callback inventory admissions.
The generator also retained the old fixed denominator. Its frozen baseline is
now extended by the three explicitly named ITEMHOLD admissions, deriving the
new denominator from that cohort rather than hand-editing generated counts.
Regenerated with
`python tools/content-schema/quest-authoring/check_quest_claim_qualification.py`.
All 297 previous records and donor pins are unchanged as decoded JSON;
only the three missing records are added, each still blocked on QUEST-QUAL-1.
The validator retains exact denominator, row-set, placement, reward, evidence
and duplicate-key checks. The seven focused tests pass, including missing-row
rejection for each new admission; `--check` passes. Governance validation and
all 129 governance tests pass. This audit repair changes no production content.

### Donor capture follow-up on ddc18edf8

The donor check reproduced `ValueError: stale donor Source capture: npc.json.gz`.
The capture includes hashes of the canonical quest index and definition shards;
the earlier quest regeneration changed the index and three shard hashes without
refreshing this dependent capture. Refreshed with the owning script's default
write mode, `python tools/content-schema/quest-authoring/quest_donor_source_authoring.py`.
Its seven changed outputs contain only propagated provenance/hash changes,
verified by recursively comparing decoded JSON against HEAD (including gzip
payloads); NPC source data, associations, semantics and counts are unchanged.
`python tools/content-schema/quest-authoring/run_checks.py --part donor` now
passes (exit 0), including the regenerated capture's strict byte comparison.
The exact unit-tests workflow command,
`python tools/content-schema/quest-authoring/run_checks.py --part unit-tests`,
passes all 1,052 tests across 93 modules on the regenerated files.

### PostgreSQL follow-up on ddc18edf8

The API for run 42316108578 identifies exactly two failed PostgreSQL jobs:
partition 22 (128231890219, registered E2E targets) and partition 1
(128231890270, library tests). Job annotations contain only exit code 100.
Individual job downloads failed through the proxy with
`Tunnel connection failed: 403 Forbidden`; an authorized direct request failed
with `[Errno 111] Connection refused`. Once the run completed, its full log
archive was available through `gh run view`'s cache. It confirms the privacy
failure below and the hotkey coalescing failure in partition 1. Local partition
reproductions use the workflow's exact duration-balanced assignments, not an
unrelated nextest hash partition.

Partition 22 reproduced locally on PostgreSQL 18.6: 40 passed, one failed,
`s17_cast_privacy_postgres_cases::s17_cast_postgres_cross_worker_consent_barrier`,
with `game durability root is not ready`. The test's 40 ms timeout borrows rather
than cancels its withdrawal future. It can suspend that future during a semantic
pass, leaving the root's single connection held when the test starts its next
operation. The test now resumes that still-pending future until its pass returns
the holder before issuing another operation. Both original timeout assertions,
both-worker invalidation/ACK requirements, persisted consent checks and media
privacy assertions remain. No production code, lease or deadline changes.

Partition 1's log contains exactly one failed case:
`hotkey_equip_postgres_coalesces_burst_without_cancelled_receipts` at
`postgres_tests/hotkey_equip.rs:557`, `Moved` versus expected `Rejected`.
The test awaited command 2's reply and a SQL equipment count before sending
3–5, allowing that round-trip to consume the native 200 ms cooldown. An unchanged
binary passes locally, confirming the failure is intermittent. The repaired
fixture queues 2–5 before polling the server and checks all four ordered replies.
It verifies command 2's one-slot equip through its immutable movement receipt
after the burst rather than inserting a SQL round-trip inside the cooldown.
The exact zero receipts for cancelled 3/4, final custody, command 6 replay and
durable reload assertions remain. Native timing and production code are unchanged.

The privacy regression passes through
`cargo nextest run --locked -p oteryn-game-server --test character_authority_postgres -E 'test(=s17_cast_privacy_postgres_cases::s17_cast_postgres_cross_worker_consent_barrier)'`;
three additional runs of that exact compiled test also pass on PostgreSQL 18.6.
The broad partition-1 diagnostic was stopped after the CI archive identified its
single failing case; 13 tests had passed and four active cases were interrupted.
That interrupted diagnostic is not reported as a passing partition or full suite.
The world compiler's `pin-check . /tmp/itemhold-ddc-final-pin` passes with digest
`6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671`.
The first repaired hotkey run passed all four burst reply assertions, then exposed
an incorrect ID in the new receipt assertion: ordinary fixtures use item 71,
whereas nested fixtures use 100. The assertion now checks Character 41, command
2, item 71, an equip operation and destination slot 5; the existing durable
reload assertion independently identifies that same item and slot.

The final hotkey regression passes three consecutive PostgreSQL runs (47.099 s,
45.981 s, 46.805 s) using the exact failing case and unchanged native timing:

```sh
cargo nextest run --locked -p oteryn-gameplay-dispatch \
  -p oteryn-game-server -p oteryn-gameplay-durability --lib --stress-count 3 \
  -E 'test(=gameplay_transport::monk_save::postgres_tests::hotkey_equip_coalescing::hotkey_equip_postgres_coalesces_burst_without_cancelled_receipts)'
```

A final jobs API enumeration still lists only partitions 1 and 22 as failed in
run 42316108578. The archive scan finds exactly the two repaired failing cases.
The reward variant packet's `reward_claim_variant_migration.py --check` also
passes after the artifact refresh. `cargo fmt --all -- --check`, the strict
qualification ledger `--check`, governance validation and 129 governance tests
pass. No timeout increases, ignored tests or production runtime changes are included.

Final changed-crate Clippy passes (exit 0):

```sh
cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets -- -D warnings
```

Both PostgreSQL regressions, the complete quest unit-tests job and donor job,
strict audit checks, fmt and pin-check are green before publication. All changes
are delivered together on the existing ITEMHOLD-Q branch.
