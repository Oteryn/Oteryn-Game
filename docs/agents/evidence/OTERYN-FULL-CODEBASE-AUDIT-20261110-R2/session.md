# Session audit — REVIEWED

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Scope: all 115 tracked files under `crates/session`, 18,698 lines, including Cargo manifest, production code, embedded tests and all separate test modules. Exact full-file ranges and SHA-256 are in `coverage-ledger.json`; substantive lane notes are retained separately. The checkpoint builder checks every range and hash against the immutable checkout. No remaining package files. REVIEWED means static source analysis completed; it does not mean tests passed or runtime was verified.

## Entry points and responsibilities

`Session::admit` consumes an established stream, writes bootstrap, validates the pre-admission acceptance/refusal, checks selected capabilities are advertised, then stages and commits the bounded join snapshot. `Session::recover` additionally proves the same logical session, increased connection generation, nonregressing server-sequence/command bounds and unchanged selection/schema. WorldId and ChannelId remain distinct typed identities.

Command APIs share one command-id/sequence owner. The next command id is consumed before writing; correlated replies and interleaved state deltas are processed by the same exchange. Feature-owned opaque gated command payloads are routed by the negotiated registry, not semantically authorized by this client. Typed adapters perform additional payload/status checks. The server remains the gameplay authority.

`service_liveness` owns idle reading through a cancellation-safe first-byte poll and bounded completion of started frames. A persistent 20-second silence deadline survives short polls, and the finite caller window is checked even under continuously readable traffic. Most public command wrappers poison the session on transport/protocol failure. A completed activity-scope transition interrupts the origin command without poisoning the adopted destination. Ordinary business refusals remain distinguishable from malformed or contradictory wire outcomes.

## Invariants, failure, replay and reconnect analysis

- Ordinary post-admission input passes envelope phase, direction and exact generation checks before typed consumption. Sequence continuity is checked with overflow-safe arithmetic. Invalid typed data can consume a sequence only on a path that makes the session unusable; it is not silently retried.
- Snapshot assembly validates bounds before growth, exact transfer id/generation/chunk order/count/total and the matching commit. The assembled body is decoded once. Replacement state, typed projections and queued events are staged before installation; malformed replacement does not partially commit.
- Same-scope replacement snapshots retain absent domains and prohibit content-generation changes. Activity destination snapshots start with fresh domain state, require the receipt-bound identity/selection/spatial data, validate an optional acknowledgement before adoption, and reset command/duplicate/probe/VIP namespaces. No origin command is automatically retried in the destination.
- Duplicate command results must identify previously reserved ids and cannot complete the current command; the history is bounded. Current results correlate exactly to the outstanding command. Malformed or unknown duplicate history fails closed.
- Entity reductions clone state, require unique identities and exact incarnations, enforce own actor/location and capacity, then swap. Appearance bindings validate dictionary/content generation, sorted nonzero item references, exact entity incarnation/appearance reference, high-water allocation, bounded bindings and context consistency.
- Map changes preserve pinned bundle/content/reset binding; container and inventory changes derive from sequenced domain updates rather than accepted replies. Depot movement waits for an inventory/container/bag revision change under deadline. Opening a depot source allocates a first view rather than replacing a nonexistent view; missing source-view delivery returns a bounded Stale result without unnecessarily ending play.
- Capability selection controls both commands and incoming typed domains, including guild/party/private/public chat variants, world Channel-list identity, status and presentation. Chat lines retain a bounded ring; the session event queue is bounded. VIP online expiry uses monotonic local deadlines. Transient notices and sound/analyser domains validate empty or permitted initial snapshots to prevent replay of past events.
- Cast observer admission/media is a separate interface with no player command methods. Framing keeps bounded partial-read state across cancellation, validates generation/epoch/monotonic media sequence, and ends after protocol failure. Its idle deadline persists across canceled reads.
- Property/training adapters check varint overflow, duplicate/unknown fields, truncation and semantic validity. Core account-achievement command10 remains separate from optional earned-notice capability8.

## Cross-module traces and hypotheses

The recovery catalogue revision hypothesis is refuted for the inspected producer path: `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs`148–225 routes admitted connections through `setup::prepare`; `serve/setup.rs`13–202 calls `snapshot::initial` and transmits a complete committed snapshot; `serve/snapshot.rs`115–195 invokes `SessionCatalog::initial`; `serve/spell_catalog.rs`1–88 constructs domain100 at revision1 for each new connection. The inspected house snapshot also invokes that initializer (`serve/house_hireling/snapshot.rs`150–202). Thus the client fresh-admission requirement of catalogue revision1 is consistent with these producers. These are partial cross-reference reads, not completion credit for gameplay/dispatch.

House handoff retains unspecified old domains through the ordinary replacement mechanism after receipt handling, unlike activity handoff. The difference alone is not a confirmed defect: the actual house producer's snapshot completeness and downstream invalidation must establish stale state survival with an effect. A targeted cross-check was requested from the client auditor; this stale-domain hypothesis is not itself confirmed. This is an integration follow-up, not an unreviewed session file.

Other hypotheses refuted by inspected guards include unbounded event accumulation, stale-generation typed consumption, replay of old audio via snapshots, premature depot local mutation, command-result acceptance for an unsent id, and treating chat rooms as gameplay World/Channel identity. External cancellation during an active command is not claimed as a production defect without a caller; subsequent correlation failure is fail-closed.

## Findings and test evidence

No new confirmed P0/P1 finding in this package. The separate handoff-order defect PS-R2-003 is now independently CONFIRMED P2; see `verify-house-handoff.md` for the actual producer/consumer proof and limits. The separately confirmed P2 device-ticket integration issue belongs to platform-client; its evidence and independent verification are in the private candidate packet and the lead report.

All 115 session files were statically reviewed, including all test source. Session tests were **not executed**. Fixtures use in-memory duplex peers and do not establish deployment, real TLS, SQL durability or production incident frequency. No full workspace/SQL tests, source fixes or repository mutations were performed.

The reviewed tests cover admission/refusal, selected dependency routes, strict sequence/generation, bounded chunking, replacement atomicity, recovery, activity/house receipt flows, entities/items, appearance dictionaries, chat, transient notices, containers, map/depot, status and typed command adapters. One coverage caveat is explicit: `src/tests/depot_map_tests.rs`201–219 passes a full encoded wire frame to `apply_state_delta`, whose interface expects the envelope payload. The assertion can therefore fail before the intended reset-binding guard and is not isolated proof of that guard. The production map path was reviewed separately. Opaque generic-route fixtures likewise test transport gating, not their feature owner's payload semantics.

## Remaining scope

Package files remaining: none (independently checked in the coverage ledger). Wider assignment continues with all tracked files in `crates/protocol-oteryn`. Cross-module house invalidation and native runtime execution remain explicit limits; neither is silently marked VERIFIED.
