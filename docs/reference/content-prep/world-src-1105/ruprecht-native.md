# Ruprecht source facts and protected producer boundary

Ruprecht is **STOPPED in the retained production patch**. The latest open-branch ownership union protects both real native NPC consumers. Source evidence and captured Item candidates are retained; the prototype writer, staging generator, package selection commands and tests have been withdrawn. Donor code is `OTS_HYPOTHESIS_ONLY` evidence, not an instruction source.

STOP (five lines):
1. Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal `201189f871d23597b82caf88fd611454a2ddefb8` establish Ruprecht's placement, 22 repeatable exchanges and their physical Item facts.
2. A real producer needs the fresh NPC_TALK consumer and the Dialogue-backed NPC's native conversation admission; source facts alone cannot grant inventory authority.
3. The latest open branch `faf8ae8700d935bca612261730aca5466526389a` protects `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs` and `crates/gameplay/npc/src/npc_talk.rs`.
4. Those consumers remain exactly main; no parallel dispatcher, unused gift helper, package-only admission, alternate mint path or claimed live Ruprecht service is retained.
5. Stop this merchant component until its owning branch permits the real consumer changes; the blocker is current file ownership, not missing offer/placement facts or an invented annual rule.

## Established source facts

Crystal `data-global/npc/ruprecht.lua:54-76,95-124` defines 22 offers: 21 token redemptions and one present-bag exchange. Present bag 6496, quantity one, becomes one Christmas token 6526; every other row spends its listed quantity of 6526 and creates one listed gift. There is no permanent claim flag or annual gate in this callback. Item 174 is **parchment**, despite the offer label `gingerbread recipe`; the source output must not be silently replaced by an invented recipe item.

The source position is `(31953,31718,7)`: Crystal `data-global/world/world-npc.xml:1522-1523` and Canary `data-otservbr-global/world/otservbr-npc.xml:1350-1351` give the centre x/y and zero relative x/y offsets, with absolute leaf z7. Centre and leaf floors must not be added together. Exact offers, XML raw URLs, revisions, SHA256 and Git blob verification are in [seasonal-continuation-sources.json](seasonal-continuation-sources.json).

The canonical Ruprecht definition is Dialogue-backed (`oteryn:dialogue.npc.ruprecht`, services empty), unlike Santa's null dialogue. Its source script installs FocusModule (`ruprecht.lua:182`) without overriding defaults. Crystal `data/npclib/npc_system/npc_handler.lua:77-82,90` establishes range four, `Greetings, |PLAYERNAME|.`, `Good bye, |PLAYERNAME|.`, and walk-away `Good bye.`; lines 348-355,386-410 substitute the player name. `modules.lua:5-6,281-300` supplies only `hi`/`hello` and `bye`/`farewell`. Canary corroboration and its own offsets are recorded in the source packet.

Crystal `data/npclib/npc.lua:13-20` makes `MsgContains` case insensitive, rejects a preceding alphanumeric word, and permits a suffix such as `yesplease`. The direct `itemsTable[message]` lookup remains case sensitive. Topic one ignores another offer name until confirmation. A native 30-second quote safety bound used in the withdrawn prototype was not a donor annual/eligibility rule.

## Retained physical evidence, not native admission

[ruprecht-native-item-sources.json](ruprecht-native-item-sources.json) records all 23 relevant IDs, fresh Canary47/Crystal201 XML line ranges, names, weights, capacities, source offer rows, appearance object IDs with zero-based byte ranges, and exact raw-file hashes. Canary `src/items/items.cpp:235-254` establishes the appearance-flag interpretation; `src/items/items.hpp:29,224,331` supplies zero-speed and stack-size defaults. Each captured raw file's Git blob matched the independently fetched pinned tree.

The 22 captured missing profiles are retained only as [ruprecht-native-item-candidates.json](ruprecht-native-item-candidates.json). Their historical `overlay_sha256` in the facts packet identifies these exact candidate bytes. They are not referenced by a production manifest or generator. The existing default native manifest had only 6500 from this service; the remaining 21 offered IDs and token 6526 lacked native policies, although all 23 keys existed in the Item key index. Existing 6500 was never overridden.

A source-bound prototype used the existing atomic consume/transform owner and its registered Craft envelope (tag 15), rather than falsely representing tokens as gold or using a once-only RewardClaim. Its conservative constraints included plain inputs, at most six carried stacks, native weights/capacity, and actual source-container output placement. That experiment established a feasible owner shape; it does not make a retained production consumer available under the latest protected-file boundary. No new wire command, capability bit, mint tag, current merchant migration, staged package or supervisor feature is retained for Ruprecht.

## Retired experiment evidence

The fully qualified controlled red retained the real source package, all 23 admitted policies, native catalogue and SQL-bound actor lease, while restoring main's fresh NPC dispatch behavior by removing only the Ruprecht prepare/apply binding. It reached the mechanic assertion `10 != 9` in `/tmp/world-src-cont/ruprecht-qualified-red.log` (174.51 seconds). `/tmp/world-src-cont/qualified-red-source.json` records the exact source hashes and restoration. Earlier incomplete-fixture failures are not the authoritative red proof.

Before that controlled backtest, the complete prototype case passed with `POSTGRES_CASE_COMPLETED RUPRECHT-EXCHANGE` after cleanup (`/tmp/world-src-cont/ruprecht-green9.log`, exit 0, 167.42 seconds). The post-restoration repeat was interrupted when the new ownership boundary arrived. These are **withdrawn experiment results**, not current-tree Ruprecht validation or a shipped mechanic. The experiment covered repeat purchases, command replay, 500-token and nested 250-token payments, capacity rollback, reverse bag exchange, gold/case/stale-authority rejection, bounded fragmentation refusal, conversation retirement and runtime SQL predicate/custody negatives.

Migration-only diagnosis replayed unmodified baseline SQL and the experimental extension in private database `world_src_ruprecht_diag_1793925278481260890`, verified runtime EXECUTE privilege, zero PUBLIC EXECUTE grants and pinned search path, and dropped the database. That extension has now been removed. Full experiment backups are outside the checkout under `/tmp/world-src-cont/ruprecht-experiment/`; package-tool copies are also under `/tmp/world-src-cont/ruprecht-retired-package/`. Final retained-tree validation is reported separately by the parent.
