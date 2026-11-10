# World bundle package audit

Input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Status: **REVIEWED**, all 11 tracked files / 2,879 lines. Native Rust/zstd tests were inspected, not run. No source changes.

## Entrypoints and invariants

The production reader/visitor and compiler writer share manifest, sector, spawn and NPC validation. Header/version/reserved fields, the whole-body digest, exact frame offsets, per-frame digest, one canonical zstd frame with checksum/content size, compressed ratio and aggregate raw limits are checked before decompression. Contiguous offsets start within the body and each checked frame preserves that invariant, so the later remaining-length subtraction is not an independent unchecked arbitrary offset. Decoded tile/entry budgets are charged before reservation. Sector order, u16 coordinates, strict canonical varints, depth progression, known attribute bits, bounded UTF-8 text and exact end-of-payload are enforced. Writer round trips ensure its emitted bytes satisfy the same decoder.

Manifest decoding rejects duplicate/unknown top-level fields and uses closed nested structures. Production build class excludes draft/skipped content. Runtime version, supported capabilities, sorted unique floors, terrain ground-speed semantics and teleport destinations are checked. Placement keys bind floor/position/top-level ordinal and are meaningful only with the bundle digest. Dropped teleport keys must actually name a top-level entry.

Spawn sources retain authored point order, validated sorted identities and bounded counts. Boss and ordinary respawn forms are separate: a zero ordinary delay requires a typed boss policy with checked interval/chance bounds and compatible runtime version. NPC placements require bounded unique keys, strict ordering, valid directions, world bounds and no shared cell. Their catalogue digest is path-sorted and length-framed. Catalogue filesystem inputs are local compiler/server inputs, not player-provided paths.

Subarea masks validate source identity, exact safe asset path, matching hashes and admitted/source counts. Area and town identities, parent relationships, default town and temple metadata are checked; row runs are sorted, disjoint and within the world. Missing coverage remains unknown. The complete inline subarea/manifest tests and separate NPC tests were examined for their actual boundary assertions. Standalone NPC tests use an explicit synthetic extent; full bundle admission separately enforces native negative floors.

Epoch layers are bounded, individually decoded/validated and cannot carry house tiles. Selection rejects unknown/overlapping layers and intersections with existing houses, remaps palette indices with checked addition and enforces total counts. Composition mutates a temporary bundle before completion. The actual boot caller (`crates/gameplay/world/src/map/boot.rs:275-345`) uses `?` before installing the composed base, so a failed temporary composition is discarded rather than installed.

## Actual consumers and counterguards

`crates/gameplay/world/src/map/mod.rs:553-642` compares the trailer to the independently supplied pinned digest before parsing, builds local state through the visitor, propagates any reader error and rechecks manifest pins before returning the world. `map/boot.rs:480-522` similarly returns temporary fact builders only after successful validation. The server NPC reader (`apps/game-server/src/node/serve/bundle_npcs.rs:305-327`) converts the NPC table only after successful full visitation. The compiler's entry-start consumer (`tools/world-bundle-compiler/src/entry_start.rs`) keeps its candidate cells local until visitation succeeds. Thus partial visitor callbacks are not evidence of an accepted partial world in these actual paths.

## Findings, unresolved hardening and limits

No new confirmed production P0/P1/P2 finding was established. One decoder-hardening question remains without an established compiler-produced trigger or production impact. Detailed candidate evidence is retained privately; it is not counted as a confirmed finding or a failed executed test.

No native bundle/zstd round trip, live world boot or runtime memory measurement ran. Tests inspected are not tests passed. Compiler, map owner and server packages retain their separate remaining-file scope; only the explicitly inspected consumer ranges receive cross-module coverage.
