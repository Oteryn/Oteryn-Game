# Protocol Oteryn audit — REVIEWED (static source review)

## Scope and evidence

Immutable input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`, local checkout `/workspace/scratch/9868b8ed6bb7/Oteryn-Game-audit-r2`.

The entire tracked `crates/protocol-oteryn` package is reviewed: **596 files**, comprising **318 Rust files, one manifest, 274 runtime/golden JSON files, one README and two binary fixtures**. The text scope contains **81,639 lines**; the binary fixtures contain **302 bytes**. No files remain. Every file has a complete line range (or explicitly marked byte range), immutable SHA-256 and substantive manual review note in `ledger.json`. `protocol-final-validation.json` and `ledger-validation.json` recheck those ranges and hashes against both the immutable Git blob and working copy.

This is a completed static source review, **not a native VERIFIED build**. Parsing files, generating metadata or reading a directory did not independently earn review credit. Each of the 137 runtime capability records and its golden counterpart was considered as a semantic pair before the supplemental validator compared them. Both binary fixtures were manually decoded. The delegated social group contains 34 files and 6,968 lines; its full evidence was independently checked before integration. Its 117 cross-reference records remain in `social-worker/cross-reference-ledger.json`; the parent worker's additional 154 cross-reference records are in `cross-reference-ledger.json`. These selective external reads do not constitute full review of their packages.

## Entrypoints and trust boundaries

`src/lib.rs` exposes the foundation envelope and typed extraction, admission, snapshot, command/replay, recovery and feature payload codecs. `build.rs` and the generated-table consumers bind registry identifiers, feature dependencies and offer memberships. The runtime registry is not itself an authorization grant: the real dispatch admission fixed point and the selected capability set govern what can be sent and accepted. The feature modules provide bounded serializers/deserializers; actual gameplay ownership, current session checks and durable receipts belong to the callers.

The main boundaries reviewed are client-to-server command decoding, server-to-client state/result decoding, initial/recovery snapshot assembly, retained command replay, content-generation indices and optional feature negotiation. Real callers were traced through Session, client runtime/UI and gameplay dispatch/durability where needed to distinguish a suspicious codec shape from a reachable integration defect. Typed World identifiers remain separate from Channel and scoped runtime identifiers. Connection generation, exact actor incarnation, item-reference generation and Character lease fences are distinct guards.

## Invariants and completed semantic groups

### Foundation, negotiation and recovery

The envelope caps, borrowed lengths, checked varints and field uniqueness were reviewed together with typed presence checks. Foundation limits include a 1 MiB envelope, a 64 KiB bootstrap payload, 16 KiB credential material, 256 wire capabilities, 128 accepted selections, bounded revisions/repeats and bounded snapshot chunks/assembly. Accepted CAPCAP-3/4 contracts were used where historical resource prose differs. Decoder tolerance of unknown same-major fields is distinguished from known-field duplication or incorrect wire types. UUID identity wrappers are nonnil v7 values and remain distinct types.

Retained results are re-enveloped for the current generation and server sequence without reinterpreting a replacement command payload as a new command. Resume maintains negotiated selection. Queue hints and bulletin extensions remain bounded metadata; neither supplies admission authority. Snapshot assembly, command status/replay boundaries, parser count limits and fuzz helpers were read with all their tests.

### Transfers, observers and live state

Activity and House transfer codecs validate role-specific identities, scope changes and receipt fields. Actual activity transitions require unchanged selected capabilities and fresh generation identity, and rerun setup snapshots. Cast/spectate codecs use role-specific selection and bounded media; live epoch and visibility permission remain caller responsibilities.

Entity and item views were reviewed for current handles, incarnation identity, ordering, page capacities, narrowing and replacement semantics. Session applies entity changes on a clone and checks player icons against visible current Players. Real map planning and handle-table replacement impose the global handle budget and rollback behavior that payload codecs alone cannot establish. Native/legacy floor conversion is explicit in shipping movement, sound, cast and chat composition.

The completed groups include map windows and movement, pacing, names/lights/clock/skulls, container trees and sorting, hotkey equipment, browse-field, world-object use, beds, push/fishing, attacks, spells, vitals, cooldowns and conditions. Candidate presentation protocols were distinguished from negotiated production protocols.

### Economic and ownership-related features

Trade, coins, Store, house management/ACL/sale/move-out/rotation/wrap/podium/hirelings, NPC services/shop, depot/search, stash/filter, mail/market, quick loot, timed items and training were reviewed with their test sources. Actual command paths bind the admitted session and current handle/fence; replay uses retained immutable receipts before fresh custody or placement checks where required. Read-only views and previews confer no mutation authority.

Forge, weapon proficiency, trained/loyalty/full skills, Wheel/Gems/Atelier, daily rewards, picker/returner offers and Premium observations were reviewed. Real owners rederive current catalogue membership, prices, vocation/access and total selection budgets, and distinguish commit uncertainty from a terminal semantic refusal. Premium freshness uses request-start monotonic time. Returner claims bind Character and World and atomically couple the inbox delivery and claim. These traces are selective evidence; the subsequent complete durability audit remains necessary.

### Progression, observations and social features

Bestiary/Charms, Animus/Bosstiary, boss activities/cooldowns/taints, difficulty/rewards, arena/Svargrond/Drome, PvP modes, Hazard, events/raids/weather, Prey, full Task Board and Soulseal entry were reviewed. Actual reward reads bind Character and World; arena advance requires authoritative credited opponent death, and activity outcome requests are checked against persisted state/deadline. Soulseal authorization is bounded, one-use and tied to current session/fence/content/expiry; a replay receipt is checked before fresh token consumption.

Titles, Cyclopedia character/combat/protection/offence, item/gallery/achievement/profile views, map/region/geometry atlases, trackers and Exiva settings were reviewed. Unknown values remain distinct from known zero observations. Combat projections expose sampling time and source revisions. Cross-character reads were traced through current-session and VIP-consent guards. Additional integration candidates remain under independent review in restricted evidence.

The delegated social review covers chat/channels, party/list, guild/bank/war, VIP/presence, reporting/typing and Team Finder. It traced shipping composition, ownership, actual mutation replay and native UI counterguards. Its report and private candidate packets retain the detailed boundaries.

### Final observational families

Analyser fact codecs strictly separate one committed Kill from any number of loot fragments, reject invalid oneofs/types/counts and enforce 64 facts, 4,096 bytes and 32 loot items per batch. The 1,024-entry pending queue drops the oldest facts with an explicit Dropped counter. Party accounting uses independent committed counters; requester prices are display valuations, and generation-matching item identity is required before using them. Actual party ownership restricts disclosure.

Achievement query input carries only a page. The actual admitted controller supplies the account. The producer derives metadata and sums from the catalogue, performs checked pagination and reads provenance-joined bounded facts. Notice refresh rechecks the exact active controller/fence around I/O and uses a key-set difference, avoiding timestamp cursor collisions. Reconnect seeds old keys silently; uncertain writes never reuse a possibly sent sequence/revision.

Highscores use a typed optional public World while retaining the admitted authority World and asking Character. The owner checks retained current/previous snapshot tokens, category and own-row identity. Global rank and stored own-page position are separate. Additional integration behavior remains under independent review.

Quest journal messages expose only content-generation indices, visible names/texts and completion flags. Queries carry no track values, claims or reward authorization. Bounds cover 1,024 listed quests, ten tracked quests, 128 missions per line, 16,384 bytes per line and 180,268 bytes per domain. The actual owner uses the existing session copy, enforces listed-only selection and preserves rate/tracked/revision continuity across reconnect. ListOnly does not replace the visible view or increment its domain revision.

## Failure, replay and reconnect behavior

Malformed or oversized untrusted payloads are refused with explicit bounded errors; semantic availability remains an owner decision. Strict decoding was not treated as proof of current authorization. Read-only query failures and unknown durable commits were distinguished. Sequence/revision changes before a possibly completed write were examined at the real emitters; reconnect replay of immutable command results was separated from snapshot replacement of observational state.

The successful activity path reconstructs the analyser domain, so omission of analyser pending flags from a generic cancellation helper does not by itself leave them stuck. The House path has a separate confirmed ordering/continuity defect and must not inherit the activity counterguard by analogy.

Current-session scope, transport, Character lease, exact actor and generation checks were followed across I/O in the meaningful cross-references. A codec-only absence of such a check is not reported as an authorization bypass. Public World selection for rankings is intentional; a selected public World is never substituted for the authority World.

## Findings and refuted hypotheses

PS-R2-003 is an independently confirmed P2 static integration finding: actual House handoff output ordering and destination continuity disagree with the Session expectation. The bounded public proof is in `verify-house-handoff.md`. No native reproduction, permanent reconnect failure or data-loss claim is made.

Other integration candidates are preserved in restricted packets and await independent adjudication. They are excluded from the confirmed register. Snapshot-domain wire-type tolerance without an established rejection requirement and downstream effect remains an unrated hardening observation.

Refuted or bounded hypotheses include shipping chat-floor mismatch, source-controlled boss victory, arbitrary other-character reward reads, client-supplied economic authority, activity-transition analyser pending state and treating every registered capability as offered. VIP-expiry display behavior remains latent because the examined native opening path is blocked by a separately confirmed client issue.

The separately confirmed remembered-device login integration issue belongs to platform-client. Its restricted source evidence remains private under SECURITY.md.

## Validation performed and limitations

Executed supplemental checks: immutable Git/worktree/SHA-256/range validation for all ledger entries; semantic comparison, dependency acyclicity and offer closure for the 137 capability pairs; an isolated extracted foundation parser probe. The prior first-six-package work executed 25 error-codes unit tests and three panic integration tests using direct Rust compilation.

**Not executed:** the native protocol crate suite, Session/TLS integration tests, SQL suites, Windows client runtime, live deployment or production replay scenarios. Test files were fully reviewed as source, including independent wire vectors, malformed/limit cases, generated property samples, bounded fuzz harnesses and registry consistency assertions. Source-test review does not imply a passing test run. No new tests were created merely to mirror implementation; no production/source files were changed.

## Remaining work

**Remaining files in `crates/protocol-oteryn`: none.** `protocol-remaining.json` is empty. The next assigned full scopes are `crates/gameplay/durability` and `apps/game-server`; selective cross-references in this report are not credited as completion of those scopes. Root remains the only remote GitHub writer.
