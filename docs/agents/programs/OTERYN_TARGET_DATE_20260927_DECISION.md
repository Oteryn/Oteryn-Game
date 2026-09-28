# Oteryn programme target date: 2026-09-27

- Date: 2026-09-27
- Status: ACCEPTED (owner decision)
- Programme: KAN-23 overview; recorded from KAN-16 / #162
- Supersedes: the "immutable post-2026-07-28 Global Tibia production-observable behavior cut" as the external target in
  `OTERYN_GLOBAL_REFERENCE_FIRST_AGENT_LAUNCH_LINES_20260909.md`,
  `OTERYN_GLOBAL_REFERENCE_FIRST_PARALLEL_GAMEPLAY_PROGRAMME_20260909.md`,
  `OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`,
  `OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md` and
  `OTV2_CONTENT_WORLD_CORRIDOR_FIXTURE_EXIT_POLICY_20260917.md`

## 1. Decision

The owner said the July date does not matter and the target is the state "na dzień dzisiejszy" (as of today).
The owner chose to apply this to the whole programme at once and to rename the world coordinate frame.

- **Target.** The external target is Global Tibia production-observable behaviour as of **2026-09-27**. The owner may
  move the date again. A move is a new decision, not a silent rolling target.
- **Pinned evidence.** Every piece of evidence stays pinned to the date, revision or commit it was read at, so a
  result can be reproduced.
- **Historical evidence.** Evidence captured for the 2026-07-28 cut stays as it was recorded. Such evidence counts for
  the new target only after it is re-read at 2026-09-27 or proven unchanged since.
- **Coordinate frame.** The world coordinate frame is renamed from `global-target-2026-07-28` to
  `global-target-2026-09-27`.

## 2. Source order for mechanics

When a mechanic is researched, sources are used in this order (owner, 2026-09-27):

| # | Source | Used for |
|---|---|---|
| 1 | tibia.com | Official news, test server changes and patch notes; for example Rotten Charge every 90 s with 15 s of activity for Bakragore, Chagorz and Vemiath. |
| 2 | TibiaWiki (Fandom) | Boss pages and quest or spoiler pages: phases, time limits, messages, fields and penalties. |
| 3 | TibiaWiki Brasil | Tables and cross-checks of values: counters, intervals, damage and durations. |
| 4 | Recordings of real Tibia | What the wiki does not state exactly: phase order, spawn positions, message-to-attack delay, field tiles, summon despawn, targeting and area reach. |
| 5 | TibiaQA, Reddit and player guides | Supporting evidence only, when the wiki lacks the full flow of an encounter. |

Canary and Crystal Server remain implementation evidence (`OTS_HYPOTHESIS_ONLY`, D30).

Access from the cloud sessions on 2026-09-27:
- Fandom and Reddit can be read.
- tibia.com and TibiaWiki Brasil answer with a Cloudflare bot check. That check is not bypassed.
- web.archive.org copies are cut off by the session egress relay.
- Recordings need a person to watch them.

## 3. Migration

| Step | Scope | State |
|---|---|---|
| 1 | This decision; coordinate frame renamed in code, tests, the NPC admission tool and its staged evidence; `content/world` regenerated. | This change |
| 2 | Monster authoring re-read at 2026-09-27: the population and batch wiki comparisons, the ability scenes, D15/D25/D32 (new D33), census and wave A restaged. | This change |
| 3a | Item evidence chain: the `target_cut` of the item current-source, field verification, continuity and promotion tools and their CI workflows. The protected current-source, field-verification, continuity and semantic-promotion evidence is re-collected at 2026-09-27 on hosted runners (same partitions and the same 69 promoted values as at 2026-07-28). Since #1064 `cw2_b1_import` takes Item promotion from the #1048 lowering pass, so the packet stays protected evidence only. | #1061 |
| 3b | G4 wiki captures still cut at 2026-07-28: the 165 exact Items, the Item Wave 1 snapshot and stage, the 252 Mounts and 133 Outfits. They are re-captured at 2026-09-27, and the materializer post-cut limits (`2026-07-28T23:59:59Z`) move with them. | Next, coordinated with the Item import owner |
| 4 | `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`: a new manifest revision that re-reads each case at the new target. | After #1019 (manifest revision 5) |

Until steps 3b and 4 land, those artefacts still say 2026-07-28. Where they differ from this decision, this decision
states the intended target.
