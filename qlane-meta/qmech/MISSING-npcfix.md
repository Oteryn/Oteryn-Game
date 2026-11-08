# NPCFIX — PARTIAL code audit

Only the entrypoint below has been checked so far. This is a missing binding in the audited shared-NPC path, not a claim that the existing fenced QuestState writer is missing. Other adapters and the remaining NPC scope still need inspection.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| The Hidden City of Beregar — Way to Beregar | Server: no Maris (Mistrock) `mistrock` progress binding in the checked shared-NPC callback/offer registries. | `content/quests/missions/quest-state.json#/quests/70/tracks/12` declares `oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/way_to_beregar`. `content/quests/shared-reward-callbacks.json` and `content/quests/shared-reward-offers.json` each have zero `oteryn:npc.maris_mistrock` rows for the `NPC_TALK/mistrock` entrypoint. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/npc/maris_mistrock.lua:68–82`, sha256 `4bcd7342494cb23d49db54e1dfd0026034a8df39e445aefb16f3a185d91d97b1`, sets this storage to1. | Partly: this checked NPC entrypoint lacks its registered transition; other quest entrypoints have not yet been audited. |

Migration range retained from NPCFIX's existing packet: **1230–1239**. No migration is proposed by this partial report.

## Additional checked NPC-mechanic gaps (PARTIAL quest mapping)

These are NPCFIX audit interactions. Their canonical quest files/steps have not yet been mapped, so no quest association is guessed. Existing Main data must remain available; unfinished new NPC actors are disabled in the shipping candidate.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Rascalio; Ghost Captain; Hoaxette; Minzy — NPC event availability/return | Server: generic conditional presence and admitted return binding | Quest/stage mapping pending. `apps/game-server/src/node/serve/bundle_npcs/seasonal.rs:69` contains explicit Santa/baker identities, not these four. `tools/world-bundle-compiler/src/project.rs:67` supplies NPC dialogue/services but no event-presence predicate. Existing dynamic actor owner is reusable. | Partly: event-specific interaction/return |
| Yselda; Zarifan; A Swan; Eustacio's Spy Rat; ... — exact-cell interactions | Server: source-qualified stationary/water admission without relaxing ordinary collision rules | Quest/stage mapping pending. `docs/architecture/reviews/OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md:42`; `tools/world-bundle-compiler/src/project.rs:67`. Existing physical PG fixture `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npcfix_fading_placement_batch.rs:171` rejected ellipsis at (33450,32805,11), `BlockSolid`, actual PG119.363s. | Partly: affected physical interaction unavailable |
| Captain Jack; Petros — conditional passages | Server: Costello permission/fines; Travora daily ticket and last-port return | Quest/stage mapping pending. `crates/gameplay/content/src/content/npc_catalogue/travel.rs:4` explicitly holds unmapped custom/quest guards. Ordinary Petros routes and carried-cheese admission do not supply the missing ticket/return state. | Partly: guarded special passages |
| Anna — tutorial continuation | Server: admitted continuation to Targuna | Quest/stage mapping pending. Checked `crates/gameplay/dispatch/src/gameplay_transport/newhaven_travel.rs`; `crates/gameplay/content/src/content/npc_catalogue/travel.rs` still holds `captain_bluebear/targuna`. No Anna continuation binding identified in those checked owners. | Partly: continuation, not existing Main selection |

Environment prerequisite, not a mechanic: all18 whole node-boot variants (six profiles × fixture/world/full_manifest) stop with `NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing` (exit2), required Platform3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05. Authored identity/ground variants remain retained but not enabled as finished stories.

## M38 implementation progress

Coordinator-assigned new migration range: **2140–2149** (historical1230-range NPCFIX migrations are unchanged). The first engine slice supplies reusable `ConditionalNpcActor`: committed availability is projected per exact actor occurrence; turning off one placement cannot remove another quest's same-key NPC. Real PG RED233.289ms -> GREEN355.733ms plus the full entities crate tests, strict Clippy and fmt passed. Event/content bindings remain work in progress.

Audit correction for the Beregar row: the native source rules `content/dialogues/quest-rules/quest-batch3-a-the_hidden_city_of_beregar.json` already include Maris's `hidden-path` and `hidden-path-repeat` branches; `NpcRuleCatalog::runtime` admits Beregar. Absence from the shared reward registries alone is not a missing native mechanic. Its new focused PG story is retained in the legacy handoff stash and is not claimed executed yet.

## Verified follow-up (2026-11-08)

- Beregar: the native `hidden-path` and `hidden-path-repeat` rules were already present;
  the actual missing integration was Maris's published source placement. The pushed
  M38 story now exercises live NPC dialogue and the fenced progress writer: eligible
  grant/reconciliation/conflicting replay/repeat passed on PostgreSQL in
  **257.887529366 s**, and the current-premium rejection case in **256.50464213 s**.
  The shared-registry absence above is not evidence of a missing native callback.
- Hoaxette's April presence now uses the committed reset calendar and exact-occurrence
  actor owner. Its real PostgreSQL exercise passed before the auto-test mount relocation;
  the relocated entry and the whole node-boot matrix still require qualification.
- A Swan's source anchor is admitted without a collision exception. Zarifan's actual
  source anchor required an explicit solid-object contact policy: the compiler's
  real-map test passed in **108.88 s**, including rejection when permission is removed.
  This policy does not grant movement through solid terrain or waive other holds.
- The exact Platform checkout is now present at the required revision. The older
  missing-checkout blocker above is obsolete; whole node-boot qualification remains pending.
- M38 remains PARTIAL: the other conditional NPC bindings, remaining exact-cell contacts,
  guarded special passages/ticket return, and Anna's explicit Targuna continuation
  are not claimed complete by these slices.
