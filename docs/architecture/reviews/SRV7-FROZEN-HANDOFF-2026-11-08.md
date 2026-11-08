# SRV7 integration handoff

The owner requested one final merge/push, followed by a branch freeze for CI fixes.
This delivery is an integration snapshot, **not a claim that all five mechanics
are complete**. The detailed donor evidence remains in the individual SRV7 reviews.

| Row | Delivery boundary |
| --- | --- |
| S06 | Source acquisition population and callers are included. The eight Spike task/Fame acquisition leaves remain disabled; their adapter has not passed its real death/report journey. Natural Stampor collection and the Chondur owned-mount continuation are not fully validated. |
| W18 | Furniture population, acquisition, transformations, and server Wrap are included. Four carried/floor Wrap PG cases and the actual Royal purchase/inbox/house/capture journey passed. The additional native opaque-Wrap UI and opened death-container descendant mutation are not included. |
| W05d | Seasonal schedules, source actors/items, and presentation work are included. Santa's full current reward distribution still needs the owner's accepted numerical rule; the donors' unreachable probability branches do not establish it. |
| W16 | Summer source entry/spawn work is included. Real framed entry and the omitted-private-child negative passed. The direct native corpse-settlement journey exposed successive MINT/generated-column and Instance Ground guards; the final narrow Ground correction still requires its fresh integration result. |
| W23 | Already done on current main: all 17 citizenship tiles and the Svargrond qualification gate. Additional focused coverage and the source review are retained. |

Only SRV7's unmerged `0992_house_furniture_population.sql` and
`0993_seasonal_events.sql` are owned by this lane. Main's migrations are preserved.
No held TMP adapters or UI patches are part of the repository delivery.

The pre-final-merge 13-case PG run executed in 597.474 seconds: eight passed and
five failed. The failures were Artist owner/guest setup, a fresh Chondur greeting,
Simon's missing real Quest catalogue, the Gnom real source route, and Summer
Instance Ground corpse provenance. Test setup corrections preserve all strict
assertions and production guards. They are not themselves end-to-end green proof.

Pre-final-merge unit batches recorded 1042 passes (one existing skip), 930 passes
and one item-effect capability failure, and 688 passes. The stale capability test
now uses the existing canonical constant. A fourth batch hit disk exhaustion
before executing; it must not be counted as a passing test run.

## Final delivery validation

The final merge incorporates origin/main `78294c081`. Completed checks:

- Canonical World: 212.014 s, semantic preservation and byte idempotence.
- Repository model: 15 simulation cases / 777 Creatures; native registration regenerated 44 outputs.
- World-tree equivalence: 131.197 s.
- Architecture: 3.832 s.
- Workspace all-target check: 490.580 s including queue.
- Clippy, all 27 changed crates, `-D warnings`: 510.794 s including queue.
- World pins: 385.304 s including queue.
- Final formatting: 20.259 s.
- Cake 9 / seasonal-ground 6 Python cases passed.

The additional broad Python tree-test driver was stopped when it entered the
unrelated monster-overlay suite; this is not a green full-suite claim. Its stale
Creature and source-binding count assertions were exposed and refreshed through
the canonical owning tool, preserving other assertions. The production tree
equivalence validator passed independently.

The final focused PostgreSQL rerun selects the five previous failures, with
actual source bundles exported by the existing fixtures and PG enabled:

```sh
cargo nextest run --locked --all-features --lib -p oteryn-gameplay-dispatch \
  --no-fail-fast --test-threads 2 -E 'test(srv7_artist_easel_actual_map_pickup_then_house_rotate) | test(srv7_chondur_historical_owned_materials_real_npc_trade_grants_mount11) | test(srv7_actual_hugo_lifetime_and_simon_compound_paid_source) | test(srv7_gnomilly_real_700_deaths_reports_xp_and_earned_gnommander_outfit) | test(srv7_rakesh_real_source_step_enters_current_allocated_instance)'
```

Changed-crate units use `cargo nextest run --locked --all-features --lib --bins
--no-fail-fast`, with each changed package selected by `-p`; PG environment
variables are removed for that unit-only run. Early-return PG library tests in
this run are not PostgreSQL execution evidence. Actual PG durations are reported
separately.

The final Quest compatibility rerun passed **302/302** tests (33.438 s total,
27.479 s execution). All four acknowledgement declarations and eleven programme
tracks reuse their exact mature owners; only Rock Steady's literal72326 remains
a new component. The two donor `+1` symbols resolve to the existing native keys.
The source linker retains all preexisting looks, effects and source records.

The last five-case PG group executed for **596.029 s**: Artist passed in
418.513 s; Simon paid continuation failed in305.014 s, Chondur's owned-mount
refusal in177.099 s, and the Spike route in177.508 s. Rakesh's4.617 s result was
a disk-exhausted database migration, not a gameplay result; it is retried after
cleanup. The earlier Artist green was179.374 s. No failing assertion was removed.

The dispatch unit run was interrupted under the owner's PREPUSH-2 cadence
exception after **702 passing and five failing test headers**. Remaining dispatch
tests and the other25 package groups are **pending CI**, not green. The five
observed failing filters are:

- `creature_damage_spell_tests::summer_phosphorus_curse_qualification_preserves_final_stats_and_rejects_drift`
- `gameplay_transport::bundle_gates_tests::a_bundle_world_seam_has_no_qualified_room`
- `gameplay_transport::bundle_gates_tests::every_gate_call_is_inventoried`
- `gameplay_transport::capabilities::tests::the_production_offered_set_is_the_registry_offered_set_and_registered`
- `gameplay_transport::charm_progression::tests::canonical_bestiary_binds_every_race_with_its_charm_points`

The first assertion still expects zero XP although the retained published-field
owner qualifies Phosphorus's final form at16000000 XP. The remaining failures
include source-seam inventory and registry/census assertions; they are reported
for the owner's single grouped CI-fix follow-up. Tests remain intact.

After disk cleanup, the same compiled Rakesh test executed again for **55.575 s**
and failed at `game_spell_corpse_entry_owner_lock`: `corpse custody mutation lacks
a qualified physical Ground root`. Thus Summer entry is demonstrated, but native
corpse settlement remains red; the persistent post-settlement MINT probe did not
execute and must not be counted as green.

The final code gate passed after the compatibility fixes: workspace all-target
check **156.654 s**, changed-Quest Clippy **6.564 s**, formatting **16.274 s**.
The other changed crates' Clippy passed in the earlier27-package run. The owning
tools regenerated the programme, endpoint owner fields, policy leaves, cosmetic
authoring/provisioning and manifest pin; world pin-check passed. The owner's
PREPUSH-2 cadence exception applies to the incomplete unit runs documented above.
This branch is frozen after its delivery push, for the owner's grouped CI fixes.
