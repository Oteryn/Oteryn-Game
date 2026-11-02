# Hireling native trade and audit extension

## Authority and economic owner

Hireling conversations use existing command 7, and category windows and purchases use existing command 8. The durable writer is the existing NPC trade owner. The selected source row, current effective House owner, current character-wide entitlement, actual named House, runtime generation and current conversation are rechecked before a new transaction. A retained window does not retain authorization. Historical receipts are read before current conversation or inventory eligibility so a replay cannot issue value again.

The source is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/npc/hireling.lua`; the generated catalogue preserves executable source-line numbers. Q7 preserves 1,376 literal buy-loot rows plus two commented placeholders. Of those literal rows, 1,375 have a positive price; item5807 has neither buy nor sell price and therefore emits no priced offer. Q8 selects physical coin payouts with native no-room refusal. Its maximum executable single price is item 43963, 5,400,000 gold. The generic Crystal ground fallback and optional AUTOBANK setting do not create additional delivery authorities here.

## Existing resource entries

The following are explicit shape mappings inside the same native owner. These changes allocate no new resource identifiers.

| Closed native shape | Touched items | Custody lines | Participants | Work units | Existing resource entries |
| --- | ---: | ---: | ---: | ---: | --- |
| Ordinary BUY | 23 | 23 | 23 | 66 | 0328–0331, original ordinary shape |
| Hireling furniture-kit BUY | 23 | 23 | 23 | 67 | 0328–0331, within expanded overall maxima |
| Hireling imbuement package BUY | 26 | 26 | 26 | 72 | 0328–0331, expanded overall maxima |
| Ordinary single SELL | 4 | 4 | 4 | 9 | 0332–0335, unchanged |
| Hireling single SELL | 9 | 9 | 9 | 19 | 1114–1117, within existing 23/23/23/66 maxima |
| Existing SELL_ALL | 23 | 23 | 23 | 66 | 1114–1117, unchanged |

A package is exactly one source present and its three source-defined child stacks. Each extra child adds one touched item, one nested custody line, one participant and two work units. A package has exactly three container expansions; other NPC trade shapes have zero. The existing BUY bank value-line bound remains one. Ordinary BUY keeps its original shape-specific 23/23/23/66 gate even though the same registered entries now admit the package maximum.

The native work formula is three per whole BURN, two per partial BURN, two per MINT and two per bank value line. A furniture kit adds one immutable origin metadata write. Hireling single SELL burns one source stack and may mint eight physical coin stacks. At 5,400,000 gold the payout is five 100-crystal stacks and one 40-crystal stack; values below that ceiling may need six crystal stacks, one platinum stack and one gold stack. The combined value of one Hireling SELL or per-definition SELL_ALL is bounded at 5,400,000 gold. A larger total returns the existing typed capacity refusal before any write; Q8 explicitly derives the output bound from the largest single source offer and retains current audit/resource limits. Every output has a fixed native item identity. The native planner checks all needed backpack slots before any BURN, receipt or audit write. An immutable replay returns the prior receipt without minting again.

The native trade byte entries 0337/0338 continue to bound ordinary Channel BUY/SELL at 27,094 payload bytes and 27,408 envelope bytes. Admitted named-House trades, packages and SELL_ALL map explicitly to the existing 1118/1119 byte entries: **34,502 payload bytes and 34,816 envelope bytes**. The runtime checks actual encoded bytes before writing. This does not promise that every artificial combination of maximum-length strings is admitted: an oversized combination is refused atomically, as in the pre-existing SELL_ALL contract.

The package D50 fixture checks every one of the 23 actual package definitions with 512-byte revisions, a 512-byte canonical House key, maximum numeric metadata, 20 coin inputs, both change denominations, the present and all three children. All-whole inputs reach exactly 26/26/26/72. A partial last input is two encoded bytes larger while using less custody/work. The largest encoded package is **28,386 payload bytes / 28,700 envelope bytes**, within 1118/1119. `hireling_package_resource_and_byte_worst_cases_are_bounded_by_existing_rows` measures these values with the real encoder, rejects a fourth child and rejects a participant above the package maximum.

## Named-House audit attribution

The existing `world_id` plus canonical `house_key` deterministically identifies the native House instance; the existing scope-generation field identifies its current authority. The pre-existing Channel field remains the House player's origin attribution. No synthetic Channel is substituted for a House fence.

| Existing event payload | Additive fields | Closed interpretation |
| --- | --- | --- |
| `OneItemNpcTradeV1` | optional `house_key`, field 17 | Required for a Hireling key; forbidden for ordinary Channel NPC trade |
| `OneItemFeeBurnV1` | optional `house_key`, field 14 | Required for Hireling service cause 472; forbidden for other causes |
| Native stash operation | optional `house_key`, field 6; optional `house_scope_ownership_generation`, field 7 | Both present with canonical House and nonzero generation, or both absent |

The durable writer reads the House key only after validating its actual instance fence. Deferred database proofs couple it to the current session, effective House owner, active Hireling and the same native transaction. Channel payloads omit these optional fields and keep their existing bytes and validation rules.

Fee entries 0326/0327 are extended to **26,298 payload bytes / 26,612 envelope bytes** for closed House cause 472. The old Channel gate remains 25,398/25,712. The 900-byte delta is 515 bytes for the largest optional House key plus 385 bytes for the already-admitted 512-byte service cause compared with the old 128-byte Charm-key fixture. The real maximum-length encoder test covers the full extension.

Stash retains its existing **34,000 payload / 36,000 envelope** ceilings. The maximum-output House test measures 22,808 payload bytes, including 20 output stacks, maximum definition/revision strings, 512-byte House key and maximum scope generation. Its additive House fields cost 526 bytes. It also rejects missing/zero generations, an invalid House namespace and unknown trailing fields.

## Executable PostgreSQL evidence

The existing mounted PostgreSQL target contains `hireling_postgres_trader_maximum_price_no_room_and_replay`, the Cook/package scenario and the Steward scenario. They use real command frames and current House admission, then assert native receipts, item custody, balances and audit rows. Trader tests the maximum executable offer, no-room refusal, a revoked Trader entitlement with an already-open window, and replay. Cook tests the Q10 distance dish 29409 at 15,000 gold, the specific dish at 90,000, the exact three-item `bash` present at 6,250, and replay. Steward obtains a real inventory handle and stash row through command 79, stows and withdraws, revokes guest ACL while retaining the session, and proves old receipts cannot reopen the live stash view.

The test names above describe authored assertions; completion of a PostgreSQL run is recorded separately in the task evidence. A run skipped because its administration URL is missing is not execution evidence.


## Source decoration kits

Crystal201189f `src/items/items.hpp:168-170` and `src/game/game.cpp:2761-2767` turn every buyable wrappable furniture target into item 23398 with its original `unWrapId`. The complete native capture finds 18 such targets, retained literally in the 1675-row UI catalogue. The private native BUY selection composes the proven 23398 facts; Ground-only target facts stay Ground-only. The existing nonstackable single-item BUY bound applies.

The same native trade transaction mints the kit, charges the existing coin/Bank owner, writes its ordinary receipt and audit, and records immutable target provenance under `house_wrap`. Optional native NPC trade audit field 18 names the same minted identity, exact source row and target definition. Existing Channel encodings omit it. Its 20-burn/three-mint worst case uses 23 participants and 67 work units, within the already extended BUY resource maxima 26/26/26/72. The real encoder test iterates every source target with maximum House/revision/numeric encodings and checks both whole and partial final coin burns. Its independently derived maximum is 26,097 payload bytes / 26,411 envelope bytes, checked against the existing 34,816-byte envelope ceiling.

Command 402 uses the existing House wrap receipt, custody and ACL owner for 23398-to-target and target-to-23398 conversion. It preserves the original item and mint transaction identity; a first unwrap requires a proven native purchase origin, while old 6355/6371 behavior retains its original historical-wrap requirement. Stale ACL, occupancy and capacity checks remain in that owner. The PG furniture scenario proves native refusal before debit when the backpack is full, purchase, immutable metadata, stale-ACL refusal, unwrap, rewrap and old-command replay without restoring obsolete custody.

`tools/content-schema/hireling/generate_wrapped_catalogue.py --check` compares the runtime target list and the 0721 SQL source rows against the complete pinned native capture. Source rows 3657/5526/25686 remain displayed with native typed refusal (ANSWER Q11); they are never classified as kits. Source 23721 is an all-loot service command and is barred from physical single-item pouch SELL.
