# OTV2-20261002-npc-enrichment-r21

```yaml
task_id: OTV2-20261002-npc-enrichment-r21
title: Enrich all 133 provisional NPCs from pinned Canary Crystal and wiki evidence
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: b991ac393e744b882bc3c03a22928770345ffecd
owner: codex-root-npc-enrichment
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/npc_materializer/bulk_enrichment.rs
  - apps/game-server/examples/npc_materializer/bulk_provisional.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/npc_bulk_enrich.py
  - tools/content-migration/test_npc_bulk_enrich.py
  - content
  - docs/agents/evidence/OTV2-20261002-npc-enrichment-r21
  - docs/agents/tasks/archive/OTV2-20261002-npc-enrichment-r21.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

Owner explicitly requests delegated completion with Canary/Crystal/wiki. D280 admits continuation of content draft1433; runtime, source-import authority, production, external paid-review dispatch and Merge Queue remain excluded. Root explicitly returns from b991 frozen VALIDATE to AUTHORING before writes, is the sole repository writer, and delegates independent scratch-only source and adapter lanes. Remote main integration remains a coordinator-owned pending disposition from5936572587, not silently completed.

PROVEN_SOURCE: closed133 public wiki lookup includes47 documented professions,86 explicitly unknown professions,133 city/location descriptions,55 actors with quest references,128 BR coordinate-bearing actors and19 BR trade-reference actors. Wyrdin’s Apprentice has a source-specific apostrophe variant proof for cached TP page; unchanged pinned TP snapshot still confirms132 identities. Source captures do not authorize native placements or economic actions.

DERIVED_SELECTED_OTERYN:42 basic dialogues receive50 greet/farewell messages and26 name/job replies from actual cited source utterances. The owner accepts approximation; selected simple native trigger mappings are concrete Oteryn choices, not proof of real-game matcher/state semantics.466 transcript observations and56 BR speech inventories remain documentary. Crystal author TODO speech remains explicitly provisional.

PROVEN_DONOR: new Crystal summer-update S'Zallar M'Andar outfit115, addons0/palette0, missing mount defaulted. Seven existing exact donor actors gain documentary ambient voice fields. Correction: Dragon Ancestor Spirit walkInterval2000/walkRadius2 were commented TODOs; selected project default becomes stationary with interval0/radius2 metadata, not a donor movement fact. New world spawn references remain documentary. Existing Ned Nobel alternative addon conflict is retained, not silently chosen.

Closed native before/after overlay pins exact packet and complete1282 predecessor tree39019038fb7fbdd77b0b2f89e23d129cd41c316ee050c7a2508de3effb5af1a5, requires all133 actors, preserves identities/references/services/status/disabled features, and validates a clone before replacing the draft. Existing append packets and historical verified predicates are unchanged. Generated successor files must come from the native materializer and existing Python exporter.

Source access: verified retained public Canary/Crystal Git trees and donor files, committed public BR API snapshot, byte-verified timestamped public TP/Tibiasecrets/Git transcript captures. Tavily returned432 quota limit; fresh BR/Tibiasecrets returned403; fresh Remote Desktop device list showed all devices offline. No browser fallback, private computer administration or repeated403 requests. No original asset files redistributed.

VALIDATION_PROVEN: all12 authoring checks pass; materializer76 tests, repository4, existing-map mechanics2, stage5, originalstage7 separately, authoring/schema27 andgovernance36. StrictClippy/format/successor/workbench pass. Full andfast generation match all11 canonical files, treec0d46eaa7380bea64e438f4d0855c582e0d2dc85d2e2adf388820ed1a263eb29. Independent source andnative-scope reviews pass. Counts remain1282 NPC/836 Dialogue,133 DATA_READY_PARTIAL; runtime-loadedNPCs0 andinteraction smoke passes0. Existing native_entry_room.json remains the sole map fixture; its mechanics do not qualify NPC interaction. The real-issuedWorld test is ignored forabsentPlatformreceipt. Formalcontrol-plane review/protectedCI/main integration/MQ remainpending. Durable receipts andsourceaccess limits: docs/agents/evidence/OTV2-20261002-npc-enrichment-r21/RESULT.md andvalidation.json.
