# S17-CAST-DESIGN — player broadcasting

## Podsumowanie po polsku

**Projekt zaakceptowany przez właściciela 2026-11-05 z decyzjami zapisanymi w sekcji 6.** Transmisja jest dozwolona w PvP, wojnach, arenach i walkach z bossami. Nie ma obowiązkowego opóźnienia: domyślnie 0 s, opcjonalnie 0–300 s ustawiane przez nadającego. Wybrane opóźnienie obejmuje wszystkie treści dostarczane widzom, także czat i metadane. Czat widzów jest pokazywany na żywo w kliencie nadającego, w oddzielnym panelu bez dostępu do komend gry.

Widz loguje konto Platform, ale nie loguje postaci. Nadający może ustawić hasło lub zaproszenie; limity pozostają 50 widzów na transmisję i 2 sesje widza na konto. Inny gracz może pojawić się w transmisji tylko po włączeniu ustawienia „allow me to be shown in casts”, domyślnie wyłączonego. Jeśli widoczny gracz nie wyraził zgody, widz otrzymuje neutralną planszę. Prywatne wiadomości, VIP, czat party/gildii, listy domowe, depozyt, sklep i dane konta nigdy nie są transmitowane.

**Pierwszy etap: wariant A, obsługa w węźle gry, z całym fan-out i I/O poza tickiem oraz limitami potwierdzonymi pomiarem.** Relay pozostaje drogą skalowania. Najpierw własny podgląd i moderacja, integracje Discord/Twitch/OBS później. Pakiety w sekcji 5 mają rozłączne ścieżki, jednego właściciela plików integracyjnych i najwyżej jedną migrację na pakiet.

Implementacja nadal ma status: S17a MISSING, S17b PARTIAL server / MISSING client, S17c MISSING. Akceptacja projektu nie oznacza uruchomienia funkcji. Brak opóźnienia i czat na żywo dopuszczają ghosting, stream-sniping i przekazywanie informacji; właściciel zaakceptował ten kompromis. Prywatność, brak komend rozgrywki od widzów, limity i audyt pozostają wymagane.

## 1. Scope, baseline and existing implementation

Status: **ACCEPTED with owner amendments, 2026-11-05**. This document records final S17 behavior. The earlier proposed competitive refusal, delay floor and prohibition on live caster chat no longer apply; no further owner decision on those points is pending.

Design only. Baseline: **origin/main / HEAD `a9db6a76a1ba375cb29104ec4a814310651c35fb`**. The checkout was fast-forwarded to that requested revision while preserving this uncommitted file. Citations are repository-relative file:line anchors at that revision. No runtime code, migration, numeric protocol allocation or external-repository mutation is included. Spell casting and ordinary in-range queries named “spectators” are unrelated to broadcasting.

| Area | Evidence on the audited revision | Consequence |
| --- | --- | --- |
| Dormant wire | `crates/protocol-oteryn/src/spectate.rs:6` reserves capability 33, envelope family 15, grant commands 34/35 and target-state domain 25; `:48` defines grant/state/frame/end/stop/keepalive messages. `docs/contracts/protocol-oteryn/v1/spectate_v1.proto:4` explicitly has no observer admission profile. | Reusable candidate codecs, not an admission contract or shipped cast service. |
| Capability negotiation | `crates/protocol-oteryn/capabilities/0033-spectate-v1.json:15` disables all three offers. `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs:3` explains offered/selected gating. | A client claiming support cannot activate the feature. |
| Network/session boundary | `crates/gameplay/dispatch/src/gameplay_transport/connection.rs:667` calls the refusal hook before player admission. `crates/gameplay/dispatch/src/gameplay_transport/spectate_session.rs:117` returns uniform unavailable and closes admission. `crates/gameplay/social/src/spectate/runtime.rs:115` refuses bootstrap. | No production viewer login. The native owner seam is not bearer authentication. |
| Broadcaster grant foundation | `crates/gameplay/social/src/spectate/grant.rs:53` creates revision-fenced CODE grants with a 16-byte CSPRNG secret; `:12` compares in constant time; `:91` rotates and retires old streams. `:18` checks session, scope, death, house, transport generation and lease changes. | Useful lifecycle foundation; CONTACTS unavailable. No delivered settings UI, description, human password flow or public directory: S17a remains MISSING. |
| Read-only viewer foundation | `crates/gameplay/social/src/spectate/session.rs:7` has an account-bound observer identity without character, actor, position, lease or command sequence; `:119` permits stop/keepalive and refuses gameplay. `spectate_session.rs:58` rejects gameplay frames before their handlers. | S17b is PARTIAL server; no actor is needed to watch, but O2 authentication/composition is absent. |
| View projection and egress | `crates/gameplay/social/src/spectate/projection.rs:19` accepts typed map, spatial, presentation and chat inputs; `:66` permits only local speech. `:80` serializes and clones frames per viewer, with 64-entry/1 MiB queues at `:98`. `spectate_session.rs:81` performs immediate nonblocking writes under its own owner lock. | Filtering foundation; configurable delay is not implemented. Bounded synchronous fan-out still scales with viewers; it must not be activated on the simulation work path. No end-to-end production publish composition was found. |
| Existing limits | `crates/gameplay/social/src/spectate/session.rs:69` caps viewers at 50/target; `:78` requires measured node capacity and caps sessions at 2/account. `runtime.rs:14` has no activation default for capacities. | Preserve these ceilings; no claim of measured production capacity. See resource-limit entries `0592-SPECTATE1-RL-01.json` through `0597-SPECTATE1-RL-06.json`. |
| Existing chat | `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs:1` handles character local/private/World-room chat; `:49` owns character chat sessions and `:79` composes guild, party and VIP owners. `crates/session/src/outcomes.rs:82` owns the player chat log; `apps/client/src/play/session_loop.rs:357` sends player chat intents. | This is player chat, not a spectator room. Reusing its routing would risk private leakage and spectator-to-game commands. S17c remains MISSING. |
| Client | `crates/session/src/lib.rs:269` requires a CharacterId for current admission; `:303` describes a gameplay-command session. `apps/client/src/play/session_loop.rs:63` initializes player VIP/guild/party/chat state. `crates/client-ui/src/features/0033-spectate/registrations/handlers.json:5` marks the feature unsupported; no broadcast viewer implementation was found. | S17b client MISSING; implement a separate viewer session and screen. `session_tests_admission_cast.rs:37` tests spells, not broadcasting. |
| Audit foundation | `crates/gameplay/social/src/spectate/audit.rs:5` defines security-sensitive P30D facts and explicitly delegates durable writing to SPECTATE-AUDIT-1; `:24` reserves capacity, `:102` acknowledges committed prefixes. | In-memory audit packets are not durable audit delivery or GM tools. |
| Existing test evidence | `crates/gameplay/social/src/spectate/projection_tests.rs:11` excludes private/room chat and rejects gameplay; `crates/gameplay/dispatch/src/gameplay_transport/spectate_session_ingress_tests.rs:5` checks refusal before player authority. | Source evidence only; these tests were not executed for this documentation task. They do not prove relay safety or activation readiness. |


The earlier [SPECTATE1 contract](../reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md) remains evidence of dormant behavior. For S17, the owner's decisions here supersede its competitive restriction and hidden-viewer-identity/no-chat presentation rules. Preserve authenticated account admission without a character, explicit grants, session/scope fencing, privacy and bounded egress. Use new capabilities rather than incompatible extensions of strict SPECTATE1 codecs. Reconcile the owning contract during implementation; this is not another owner approval gate.

Preserve ADR-0001/ADR-0003, distinct WorldId/ChannelId, Game-owned session authority and the [cross-repository policy](../../agents/CROSS_REPO_CONTRACTS.md). Platform owns account identity and account controls; Game owns character consent and committed visibility. Platform and later relays cannot acquire character leases or issue canonical GameSessionIds.

## 2. Threat model and retained safeguards

Adversaries include colluding broadcasters/viewers, bot accounts, compromised clients, password guessers, harassers, malicious integration operators and resource-exhaustion attackers. Protect private data, nonconsenting players, simulation latency, account security and moderation evidence. Treat viewer text, titles, credentials and frames as untrusted. Viewers are read-only with respect to gameplay, while explicitly allowed to submit cast-chat data.

**Accepted residual risk:** zero-delay competitive viewing and live viewer-to-caster chat permit ghosting, stream-sniping, tactical coaching and covert communication. Do not claim S17 prevents them or reintroduce a competitive refusal/delay floor as a release gate. The owner's reason is that OBS/Discord streams can already be live. Modified clients, external voice, screenshots and alternate accounts also prevent an absolute abuse-prevention guarantee. Privacy filtering, command isolation, moderation, limits and audit apply in every mode.

### 2.1 Eligibility, consent and lifecycle

| Situation | Accepted rule |
| --- | --- |
| PvE, PvP, wars, arenas, bosses and raids | Casting is allowed. PvP flags, combat locks, war membership and encounter/competitive classification do not themselves refuse or end a cast. |
| Bystander consent | Each character has **“allow me to be shown in casts”**, default **off**. This setting does not implicitly opt in other characters/accounts, party, guild, VIP or contacts. Explicit CastStart is the caster's consent to their own view. |
| Visible nonconsenting player | Show a neutral slate for the entire world view; no partial map/entity/name/position/marker/projectile/effect may expose that player. Hidden/off-screen players are never included and must not trigger a slate revealing their presence. |
| Missing/stale consent facts | Treat the affected visible player as not opted in. Use revisioned Game-owned settings and committed interest-set facts, never consent bits supplied by the caster/viewer. |
| Consent withdrawal | Fence queued projections, purge affected unreleased data, clear the viewer scene and show a neutral slate. Resume only from a fresh eligible snapshot subject to the chosen delay. Already delivered bytes cannot be recalled. |
| Scope/channel/instance/node/house transition | End the old grant under existing identity/scope fencing; no automatic following. A fresh explicit start at the destination is allowed, including PvP/arena/boss destinations. This is not a competitive prohibition. House lists and private information stay excluded. |
| Death, logout, lease/session/connection generation change, revoke or restart | End the grant/viewers and discard buffers. Require fresh explicit start. Watching never keeps a character or scope alive. |

Check consent before a sanitized projection leaves the trusted cast worker; never send raw player packets for viewer-side masking. If safe attribution cannot be established, slate/purge the whole projection. Explain inclusion and recording risks in the setting, and support withdrawal without affecting gameplay. Notices cannot expose hidden actors or viewer account/alternate identities. Terrain recognition and deliberate visual signalling remain residual risks.

### 2.2 Optional delay: 0–300 seconds, default 0

The caster selects integer **D in 0–300 seconds**, default **0**. There is no minimum positive delay; never silently turn 0 into a forced delay. Validate the range on client and server. At D=0, eligible content is sent as soon as normal worker/transport scheduling permits, without artificial warm-up.

For D>0, each item becomes deliverable no earlier than capture/acceptance time plus D. This covers **everything viewers receive as cast content**: initial/replacement snapshots, deltas, effects, slates, spectator chat, caster replies, viewer lists/counts, titles/descriptions, thumbnails and discovery/overlay metadata. Viewer chat is timestamped at acceptance, replies at submission. No live metadata/chat endpoint bypasses D. Authentication, keepalive and generic transport acknowledgements contain no cast content and are not buffered media.

The caster receives accepted viewer chat **live**, independent of D; the viewer-facing copy waits D. This asymmetry is intentional. Moderators watching a cast receive content with the same D; authorized controls do not expose another live world feed. Stop, kick, ban, consent withdrawal and revocation interrupt delivery immediately rather than draining the old buffer. Immediate terminal messages contain only a generic reason and no fresh cast content.

Changing D starts a new stream epoch, drops old queues and clears client projection. Increasing D waits for a fresh baseline to age; decreasing to 0 permits a new live baseline. Delayed joins/resyncs use coherent already-aged keyframes (initial proposal every 5 seconds) plus ordered deltas; otherwise show warm-up. No on-demand current snapshot bypass. Chunk snapshots within registered budgets; keyframes/chat/metadata share the same age and byte accounting. Overflow ends the affected cast/viewer rather than changing D, dropping privacy checks or fast-forwarding to live. Restart repeats warm-up only for D>0. In-node age uses monotonic time; later cross-host delivery needs authenticated timestamps and bounded clock uncertainty.

### 2.3 Information boundary

Build an allowlisted projection from post-commit target-visible facts, never mirror the player wire stream, framebuffer or UI. Allowed: eligible visible terrain/map items/creatures, public appearance/health presentation and approved effects. Use cast-local handles; strip stable character/account IDs and debug overlays. Only presentation-level caster bars are considered, not private combat telemetry or exact private statistics.

**Never stream:** private messages, VIP/contact lists or presence, party/guild/private/World-room chat, house access/guest lists, inventory/containers/equipment details, depot/stash/inbox, NPC dialogue/trade, market, store/payment screens, account/profile/email/security data, credentials/tickets, typed inputs, command results, private notifications, GM channels, hidden positions or unreleased analytics. Private UI stays hidden at D=0, during competitive play, and even when all visible players opted in.

Keep the conservative omission of ordinary local speech and player-authored signs/item text; dedicated cast chat is the allowed text channel. Unknown fields, presentation subtypes and content labels are excluded by default. Codec registration is not streaming permission. Snapshot/delta filters, resyncs, map/appearance labels, tooltips, minimap, screenshots, logs and crash reports obey the same policy. A slate clears the scene; resumption requires a fresh eligible baseline. No free camera, historical seek, replay download, remote entity inspection or minimap search. Titles/descriptions are moderated plain text. Later official external export uses this same projection and chosen D.

### 2.4 Admission, live chat and moderation

Viewers authenticate logged-in **Platform accounts without selecting/logging a character**. No anonymous viewing. Public discovery still requires account authentication for viewing. Optional cast password/invite is additional access control, never account-password login. Platform owns salted adaptive password verifiers, a mature upstream KDF, bounded workers and global guessing budgets. No raw reusable password reaches Game; passwords/tickets never appear in URLs/logs/analytics. Invite tokens retain at least 128 bits of entropy and are stored as verifiers. Password change/invite rotation revokes existing tickets/sessions.

Gateway issues purpose-separated, single-use, short-lived observer material after access/ban checks, bound to account, cast ID/epoch, grant revision, selected node, audience, policy revision and expiry. Atomically consume replay state. Game creates its observer session ID/generation, without actor, lease, position or gameplay sequence. Player admission rejects observer material. Wrong password, unknown cast, stale grant, banned viewer and expired/replayed ticket return uniform unavailable with bounded timing.

Retain **50 viewers/cast** and **2 concurrent viewer sessions/account globally**, atomically reserved across nodes and released on timeout/disconnect/revoke. Caster may lower the cast cap. Proposed abuse limits: 5 join/password attempts/minute/account, 20/minute/IP plus a node-wide budget. IP checks supplement identity and tolerate NAT; Platform owns account-creation abuse controls. Paginate/cache discovery with a 5-second refresh floor; all displayed cast metadata follows D.

Cast chat is a separate bounded room, not player local/private/party/guild chat. **Accepted viewer messages appear live in a dedicated panel in the caster's game client, even at D=300.** Caster replies use that panel; viewer-facing messages follow D. Chat never enters ChatIntent, NPC dialogue, command dispatch, console/script execution or bot APIs. Clicking text cannot invoke gameplay. No whisper, item links, embeds, files, URLs, voice, slash-command execution or external chat bridge at launch.

Text limit proposal: 256 UTF-8 bytes, NFC normalization, reject controls/bidi overrides and render escaped plain text. Per-account bucket: 1 message/3 seconds, burst 3; also 20 messages/second/room and node byte/message budgets. Bound room history, viewer queues and caster display queue independently. Overflow drops/throttles chat, never blocks caster tick/gameplay connection. Rate-limit join/leave, reports, titles and moderation actions.

Assign stable cast-local handles such as Viewer-7K2P; no user-chosen character/staff impersonation or public account/alternate linkage. **Caster and authorized moderators can mute/unmute and kick.** Retain report/ban facilities: owner-scoped account bans persist across epochs; Platform-wide bans are staff-owned. Muted users can watch but cannot speak; kick closes the session, ban prevents rejoin. Caster UI exposes actionable opaque handles; only authorized staff resolve them to accounts. Moderator grants are scoped, expiring and revocable; no Game powers/password visibility. Audit actor, subject, reason, expiry and action; authorize/mutate atomically with idempotency keys.

Allowlisted viewer inputs: cast-chat text, report, leave and keepalive; separately authorized moderation controls. **Reject every gameplay envelope, unknown opcode, nested command and wrong-direction message before any world-mutating handler.** Caster gameplay authority remains independent: viewer text receipt never triggers a player command. Viewers have no actor, lease, interest membership, presence, targeting, trade, rewards or encounter participation.

Movement/outfits and chat can be covert/tactical channels. Live chat and zero-delay viewing are accepted: use rate limits, reports, mute/kick/ban, audited GM investigation and sanctions for harassment/automation abuse. Do not promise anti-ghosting protection or add a playing-account restriction that negates the accepted live-cast behavior.

### 2.5 Capacity, audit and operations

The simulation owner emits one bounded immutable committed-facts reference per cast commit into a nonblocking queue. A **dedicated off-tick cast worker** performs filtering/encoding/compression, optional delay storage, chat fan-out and network I/O. No world/target lock across those operations, no per-viewer world reads and no viewer traffic in the gameplay command queue. Existing synchronous SPECTATE1 fan-out must not become the tick-side publish operation. Cast overload cannot pause its target.

Option A retains 50/cast and 2/account plus a **measured node-wide cap**. Qualification proposals, not measured production facts: 4 casts/node, 100 viewer sessions/node, 128 KiB/s encoded cast-content budget/cast including chat/metadata, 64-entry/256 KiB publish queue/cast, 64-entry/1 MiB egress queue/viewer. Activate with the lower measured limits; unmeasured capacity means casting disabled. Encode once/share frame references; disconnect slow viewers without slowing the caster.

Support the full D=0–300 range. At 128 KiB/s, 300 seconds uses 37.5 MiB/cast before indexes/snapshots; reserve a bounded **48 MiB shared delay ring/cast**, 192 MiB for four casts. D=0 bypasses age storage except bounded resync/egress history. Admit only if selected D fits resources; never silently clamp it. At 100 viewers, ceiling egress is 12.5 MiB/s plus at least 25% transport headroom. Measure CPU, snapshot bursts and shared-host network; include chat history, viewer queues and terminal slots in memory admission. No media database or replay disk.

Cap TLS/handshake concurrency, connections, frame sizes, decompression, discovery/password requests, limiter entries and control work. Bound timeouts and rate-limit before expensive processing. Edge DDoS protection covers observer ingress and Platform. Worker/socket/CPU/network budgets isolate viewers from simulation. Exhaustion refuses new work or ends the affected cast. If measured isolation cannot meet the Game tick budget, use option B before raising caps; never weaken privacy or selected D.

Consent/revoke changes fence final egress by revision and purge unsent affected data. Serialize the in-node revocation barrier with nonblocking final writes under the cast owner, never the world lock. No new writes after the barrier; in-flight/delivered bytes cannot be recalled. Platform policy leases expire within a proposed 5 seconds and fail closed; measure that distributed bound. Terminal output is generic, without private reason/position.

Audit grant start/end, admissions/end/refusal aggregates, consent changes, password/invite rotation, policy revision, moderator changes, mute/kick/ban/report, overload and staff access. Persist UUIDv7 identities with incarnation/ordinal deduplication and committed-prefix acknowledgement through the existing outbox discipline. Reserve terminal capacity before admission. Never log secrets, exact positions or excluded data. Audit failure stops new admission and ends affected casts while reserved terminal facts remain deliverable; gameplay continues.

Security retention: 30 days (existing P30D); restricted moderation chat evidence: 7 days by default, with documented case holds/deletion/access audit. No routine media recording; only bounded reported evidence under privacy policy. GM tools inspect cast/delay/resource health, paged opaque handles and authorized restriction reasons; resolve reports, revoke grants/moderators, silence rooms, sanction accounts and operate node/world/global kill switches. Staff access is audited, with no extra private-world feed. Metrics cover tick p99, queues/age, drops, memory/egress and moderation backlog, without position/account labels. Moderation/policy outage disables chat/admission and expires leases without changing character gameplay.

## 3. Architecture choices and integration limits

**Choose A for the first implementation:** default-live viewing reuses the dormant in-node observer/lifecycle foundation without a new service.
Dedicated off-tick workers provide filtering, optional 0–300 s buffering, chat and bounded fan-out.
Activate only within measured node limits; use B when isolation or scale requires it.
Own viewing and moderation precede external integrations.

| Option | What it allows and authentication | Cost / abuse risk | Delivery |
| --- | --- | --- | --- |
| A. In-node | Gateway account observer admission; Game-owned observer ID/generation without character. Native renderer consumes sanitized content. | Lowest deployment cost; TLS/buffers/reconnect storms share host resources. Worker quotas/node caps required. Tactical information/live chat are accepted risks. | First implementation within measured limits. |
| B. Relay | Mutually authenticated filtered Game feed; release gate applies chosen D, including 0. Platform routes purpose-bound viewer tickets to relay. Relay identity cannot impersonate GameSessionId. | Added service/egress/operations and cross-host clock/revoke contracts; better viewer-load isolation. Relay never receives private data/world-write authority. | Scaling option, not a prerequisite. Preserve a versioned feed seam. |
| C1. Discord bot/Activity | Bot posts opted-in guild announcement/link. Activity could host a web viewer after renderer work. OAuth/SDK linkage stays in Platform. Normal bot APIs are not a general Go Live screen-share API; native screen share is user initiated. | Cheap but rate-limited announcements; our hosting/bandwidth/moderation for Activity. Guild logs/previews/voice/recording escape controls. No self-bots/account automation. | Later public links, then Activity if justified; no bot-video promise. |
| C2. Twitch extension/overlay | Approved sandboxed UI around a broadcast, not video transport for Game events. Broadcaster authorizes installation; backend validates extension JWTs/scoped OAuth. OBS browser overlay is another output. | Review/hosting/API costs; Twitch CDN does not carry our event API for free. VODs/clips/chat retain information. Twitch identity alone grants no Oteryn access. | Later safe metadata/overlay using D. No viewer-to-game actions/rewards/chat bridge. |
| C3. OBS capture → RTMP/RTMPS | User captures a dedicated safe cast window/browser source, authenticating to destination by stream key/OAuth; RTMPS where supported. Safe window uses filtered projection and chosen D, including 0. | User encoding/upload; managed encoding adds compute/secret custody. Whole-desktop/player capture bypasses UI filtering/delay. | Later safe-window guidance, no launch server encoder. Keys stay in OBS/destination, never Game. |

External publication is opt-in for public casts with recording/clips consent. Public streams cannot enforce Oteryn passwords/viewer caps: no private/passworded export via public embeds/previews. External tokens stay in Platform and confer no Game authority. Recheck terms, review requirements, quotas and prices before implementing. Integrations are not a prerequisite for the owner's D=0 decision.

Official references: [Discord Activities](https://discord.com/developers/docs/activities/overview), [Discord OAuth2](https://discord.com/developers/docs/topics/oauth2), [Twitch Extensions](https://dev.twitch.tv/docs/extensions/), [Twitch authentication](https://dev.twitch.tv/docs/authentication/), [OBS sources](https://obsproject.com/kb/sources-guide). In the initial audit Twitch docs were reachable; Discord returned 403 and was not bypassed. Discord details remain implementation assumptions to verify. No integration credentials/account access were used.

## 4. Ownership and protocol shape

Content: Game committed visibility → off-tick privacy filter → optional D buffer → bounded observer fan-out → read-only renderer. Chat: authenticated viewer → cast-chat validation → **live caster UI**, with a separate copy through D to viewers. Platform account access/moderator policy feeds cast controls. None is a route to gameplay commands.

**Game:** character consent, committed visibility, ephemeral grants/epochs, target session/generation/scope fencing, observer sessions, workers/delay/fan-out, chat/moderation execution, durable Game audit and kill switches. No durable cast-on flag; reconnect/restart requires explicit start. Platform preferences cannot bypass privacy/consent.

**Platform:** account authentication/preferences/discoverability, password/invite verifier, global viewer quotas, owner-scoped bans/moderator grants/account sanctions, Gateway observer routing/admission. Later OAuth links belong here. Platform does not own character visibility truth. API/storage work requires a separately authorized Platform task.

**Client:** pre-character Watch and separate CastViewerSession without movement/spell/item/player-chat methods; delay/warm-up/slate UI; character-consent toggle; caster start/stop/access/delay/viewer-cap settings; **live cast-chat panel in the playing UI**, replies and mute/kick/moderation. Viewer mode never initializes private panels. Failure never reconnects as a character.

Defaults: casting off, D=0, optional password/invite and chat toggle. Title 80 UTF-8 bytes, description 256, under chat Unicode/link rules. Display effective D/cap before start. Bystander consent is character-owned, default false, revision-fenced and independent of account cast preferences. Caster chat shows only room text/opaque handles, no private UI data.

| Boundary | Logical messages/facts |
| --- | --- |
| Character setting | CastConsentSet{expected_revision, allow_shown} → CastConsentState{revision, allow_shown}; current session-generation authority, no implicit alternate-character change. |
| Caster control | CastStart{expected_revision, preference_revision, delay_seconds, viewer_cap, audience}; CastStop{expected_revision}; CastUpdate{expected_revision, delay_seconds, chat_enabled}; CastState{active, revision, epoch, effective_delay, count, generic_status}. Delay change resets epoch. Password/invite changes use Platform. |
| Gateway → Game | CastViewerAdmit{one_time_material} → CastAccepted{observer_session_id, generation, epoch, selected_capabilities, policy_revision, effective_delay, limits, content_revision}; no character/lease; player admission rejects this purpose. |
| Internal worker feed | CastBegin{cast_id, epoch, target_generation, grant_revision, scope_binding, policy_revision, projection_version, content_revision, delay_seconds}; CastFrame{epoch, sequence, capture_time, release_not_before, base_revision, revision, kind, payload}; CastEnd{epoch, generic_reason}. Internal identities stay internal. |
| Game → viewer | Sanitized begin, atomic snapshot commit and ordered delta/effect/chat/metadata, all under D. Cast-local handles, no target/scope IDs. Refuse mismatched bases/epochs; terminal end clears state. |
| Cast chat/control | CastChatSend{text, client_nonce}, CastChatLine{message_id, handle, text}, CastCasterChatLine{message_id, handle, text}, CastReply{text, client_nonce}, CastReport{message_id, reason}, CastLeave, Keepalive; authorized CastModerate{action, opaque_subject, expiry, reason, expected_revision, idempotency_key}. CasterChatLine is live; viewer ChatLine waits D. |

New optional capability IDs: **CAST_VIEW_V1, CAST_CONTROL_V1, CAST_CHAT_V1, CAST_CONSENT_V1**. Observer chat requires view; caster chat requires control; consent is independently selectable by any player. These are registered capability IDs, not a new bitmask. **All new capability/envelope/command/domain/payload IDs: next free at integration.** P0 owns the logical schemas; P7 owns numeric assignment/registry generation, with P0 alone applying any needed codec placeholder changes.

Keep SPECTATE_V1 (33) unoffered and preserve its 34/35/25 reservations/strict codecs. No legacy/bootstrap fallback. Reuse internals only where semantics agree: competitive classification must not set S17 observable=false. Missing consent/access policy still fails closed. Gate server output/client input by negotiated capability; view never implies control/chat. Pin producer/consumer revisions and Platform profile in the existing cross-repository lock at integration; this design alone claims no runtime compatibility.

## 5. Implementation packages: disjoint files and ordered integration

Each package has one writer and **at most one migration**. Paths below are proposed new files/subtrees unless marked existing; owning a subtree does not grant ownership of its parent. No shared edits to code, tests, module mounts, manifests, registries or generated outputs. **P7 alone owns shared composition/hot files**, through separate repository-local Game and Platform integration tasks. A Game worker receives no cross-repository write authority.

Order below is a delivery sequence, not authorization to dispatch agents. P0 freezes logical interfaces before consumers. Common-type changes return to P0; feature packages use isolated tests until P7 supplies mounts. Unmounted components remain explicitly PARTIAL; only integrated E2E proves a playable feature.

### P0 — Cast contracts and fixtures

- **Order:** 1; prerequisite for all packages.
- **Owned Game paths:** `crates/protocol-oteryn/src/cast.rs`, `crates/protocol-oteryn/src/cast/`, `crates/protocol-oteryn/tests/s17_cast_contract.rs`, `docs/contracts/protocol-oteryn/v1/cast/`, `docs/contracts/game-events/v1/cast/`, `docs/contracts/resource-limits/candidates/S17-CAST.md`; existing `docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md` for reconciliation. No root-module/central-registry edits.
- **Wire IDs:** all section 4 logical messages/capabilities; numeric values **next free at integration**, assigned by P7. P0 owns source fixtures/placeholders; P7 owns generated central outputs.
- **Migration:** none.
- **Tests:** strict/bounded codecs, unknown/repeated/malformed fields, directions, D=0/300 acceptance and invalid range refusal, snapshot/delta/epoch fixtures, consent default false, live-caster versus delayed-viewer chat and capability dependencies.

### P1 — Character consent persistence and rules

- **Order:** 2, after P0; before the next numbered migration package.
- **Owned Game paths:** `crates/gameplay/social/src/cast_consent/`; `crates/gameplay/durability/src/durability/cast_consent.rs` and `crates/gameplay/durability/src/durability/cast_consent/`; `apps/game-server/migrations/0966_cast_character_consent.sql`; `apps/game-server/tests/support/authority_cases/s17_cast_consent_postgres_cases.rs`. P7 owns mounts/routing.
- **Wire IDs:** consumes CastConsentSet/State and CAST_CONSENT_V1, **next free at integration**; no independent ID assignment.
- **Migration:** **0966 only if needed** for default-false revisioned character consent; reuse suitable existing character-preference storage otherwise. One field/record with session fencing, bounded lookup and least-privilege grants; no account-preference duplication.
- **Tests:** old/new character default false, persistence/toggle, stale/session-generation refusal, alternate-character independence, cross-node invalidation and withdrawal purge, restart and SQL grants. Use the auto-mounted case file, never the shared test target.

### P2 — Durable cast audit adapter

- **Order:** 3, after P1; before the Platform numbered schema package.
- **Owned Game paths:** `crates/gameplay/durability/src/durability/cast_audit.rs` and `crates/gameplay/durability/src/durability/cast_audit/`; `apps/game-server/migrations/0967_cast_audit_outbox.sql`; `apps/game-server/tests/support/authority_cases/s17_cast_audit_postgres_cases.rs`. No edits to consent/dormant audit sources; use an adapter.
- **Wire IDs:** consumes cast audit payload; new event/payload IDs **next free at integration**, centrally registered by P7.
- **Migration:** **0967 only if needed** to extend existing outbox/event support; no parallel audit service/table if compatible storage already exists.
- **Tests:** UUIDv7 identity, incarnation/ordinal deduplication, committed-prefix ACK, terminal reservation, crash/retry, retention/redaction, bounded queries and role grants. No success claim before durable prefix commit.

### P3 — Platform account controls and observer admission

- **Order:** 4, after P0–P2 contract/migration decisions; separate authorized Platform task.
- **Owned Platform paths (new proposals):** `app/Casts/`, `services/game-gateway/internal/cast/`, `routes/modules/cast.php`, `tests/Feature/Cast/`; one `database/migrations/<timestamp>_0968_cast_account_controls.php` if needed. Tests/testdata within the new Gateway subtree belong here. Existing routes/providers/Gateway entry/config/manifests belong to P7. No Game source ownership.
- **Wire IDs:** observer-purpose profile and CAST_* contract; new profile/error IDs **next free at integration** with the Platform owner, not selected independently.
- **Migration:** **0968 only if needed**, one forward-only Platform migration for cast account defaults/access verifiers/moderator grants/owner-scoped bans if existing stores cannot represent them. No character consent or cast-on persistence. Platform uses timestamped Laravel migrations: 0968 is the owner-assigned suffix/reservation, not a Game SQL migration or a substitute timestamp.
- **Tests:** account required/no character login, password/invite rotation and KDF budgets, uniform refusals, purpose/audience/node/grant/epoch/expiry/replay checks, atomic global 2/account quota/release, bans/moderator expiry, default D=0 and metadata following D.

Platform layout was read only at `92f270a56327bc96bc440b2a84e5dbb06d9124e0`: PHP `app/`, timestamped `database/migrations/`, `routes/modules/` and Go `services/game-gateway/` exist. Cast leaf paths above are proposals, not existing implementation. Inspect/pin actual producer code before implementation; this task performs no Platform writes.

### P4 — In-node runtime, privacy, optional delay and live chat

- **Order:** 5, after P0–P3 interfaces; before production integration.
- **Owned Game paths:** `crates/gameplay/social/src/cast/`, `crates/gameplay/dispatch/src/gameplay_transport/cast/`, `apps/game-server/tests/support/authority_cases/s17_cast_runtime_postgres_cases.rs`. P1 owns separate cast_consent, P2 audit adapter; P4 does not edit old spectate, connection routing or roots.
- **Wire IDs:** consumes CastStart/Stop/Update/State, observer begin/frame/end, chat/reply/report/moderation and CAST_VIEW/CONTROL/CHAT; **next free at integration**.
- **Migration:** none.
- **Tests:** PvP/war/arena/boss casting allowed; bystander-off slate/on inclusion; hidden actors do not leak presence; all forbidden categories absent from bytes; snapshot/delta/resync equivalence; fake-clock D=0/1/300 and invalid ranges; every viewer-content type follows D, caster text live at D=300; epoch reset, consent withdrawal, revoke-during-write and lifecycle ends. Mute/kick/ban, moderator races, Unicode/spoof/XSS, rate/byte limits, opcode rejection and bounded shared-frame fan-out never mutate/block target gameplay.

### P5 — Native viewer, caster settings/live chat and consent UI

- **Order:** 6, after P0 plus P1/P3/P4 interfaces; complete before P7.
- **Owned Game paths:** `crates/session/src/cast/`; new cast-specific feature directories under `crates/client-runtime/src/features/`, `crates/client-ui/src/features/`, `apps/client/src/features/`; `apps/client/src/cast_watch/`. Directory prefixes use P7's allocated IDs; all cast client concerns stay in this package. Existing unsupported stubs/shared orchestrators are P7-only.
- **Wire IDs:** consumes CAST_VIEW/CONTROL/CHAT/CONSENT and messages, **next free at integration**; no registry/golden edits.
- **Migration:** none.
- **Tests:** account-to-Watch without CharacterId/actor, no viewer gameplay API/route, slate clears scene/no private panels, default-false consent toggle, casting off/D=0 defaults and 0–300 validation, warm-up/resync/reconnect. **Live viewer text in caster playing client at D=300**, reply/mute/kick/moderator actions, delayed viewer copies/metadata, unknown payload rejection and no character-login fallback. Use feature-local tests/fixtures; shared event/orchestration mounts belong to P7.

### P6 — Qualification and abuse scenarios

- **Order:** 7, after P4/P5 interfaces; production tests run on P7's composed tree.
- **Owned Game paths:** `tools/synthetic-client-harness/src/cast_s17/`, `tools/synthetic-client-harness/tests/cast_s17.rs`, `docs/architecture/design/evidence/S17-CAST/` for small measured results/hardware/budget descriptions. No shared harness library/manifest edits.
- **Wire IDs:** consumes frozen contracts, **next free at integration**; no new IDs.
- **Migration:** none.
- **Tests:** real Gateway/node/client observer login, 50/cast and global 2/account plus measured node cap, TLS/password/reconnect storms, slow readers, memory/egress/tick-p99 at max cast/chat load and D=300, GM revoke/kill switch, audit/policy outage, competitive casts, consent withdrawal, malicious gameplay frames, live caster chat and delayed viewer content at D=0 and positive values. Fix hardware/accepted Game tick budget before measuring, not after results.

### P7 — Sole composition, numeric allocation and activation

- **Order:** 8, after P0–P6; compose, run P6 on candidate and then activate. Earlier bounded integration mounts may be prepared by this owner, never by feature packages.
- **Owned existing Game hot files:** protocol/session/social/durability/dispatch module roots/re-exports; `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` and `connection/routing/`; capability offer/selection composition; node/serve/config composition in `apps/game-server/src/` excluding package-owned leaves; client entry/play/session-event composition and existing 0033-spectate unsupported stubs; relevant Cargo manifests/`Cargo.lock`; harness roots/shared test mounts. Own all new central capability/command/domain/event fragments, generated tables/goldens/resource-limit registrations and `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`. No P0–P6 leaf source/migration ownership.
- **Owned Platform hot files by its repository-local counterpart:** existing providers/root routes/Gateway entry/route composition, config/deploy/manifests/locks required to mount P3. Record exact files after producer inspection before allocation; P3 never edits them. Game observer deploy/config templates are P7-only. No CI/protection change is assumed.
- **Wire IDs:** sole allocation of **next free at integration** across capabilities/envelopes/commands/domains/events/profile identifiers, checked against then-current main/open allocations. Preserve legacy 33/34/35/25. Generate central outputs once; P0 alone updates codec numeric placeholders if required, so no shared-file write ownership is introduced.
- **Migration:** none; run only P1/P2/P3-owned migrations.
- **Tests:** real-dispatch P6, negotiation/old-client refusal, no observer-to-player bootstrap, simultaneous live caster chat and gameplay, production publish callers, settings revoke reaches egress, D=0/300, rollback/GM tools. Run changed-path checks/PostgreSQL tests; require measured caps, pinned contracts and durable audit before offers.

**Migration rule:** only **0966 → 0967 → 0968**, in that order, bound to P1/P2/P3 respectively, and only if each package needs a schema change. Unused reservations create no empty migration and are not recycled by another package. At most three migrations, each owned by a different package; zero if existing stores suffice. Check occupancy before implementation; collisions return to the allocator, never silently take another number. No 0966/0967/0968 SQL file exists at this Game baseline; open allocations are not certified here. This design adds **zero migrations**.

**No shared hot files:** P0 contracts, P1 consent, P2 audit, P3 Platform leaf code, P4 server cast leaf code, P5 all cast client leaf code and P6 qualification leaf code are disjoint. Shared roots/manifests/central and generated registries/connection-client-Gateway routing/deploy/shared mounts are P7-only. Missing integration seams return to P7. Keep registrations sorted and use existing auto-mounted tests; feature packages never hand-edit shared/generated destinations.

**Rollout:** dormant components first; compose pinned Platform/Game/client revisions, qualify internal cohort with default D=0, competitive casting and live chat, then bounded native rollout. The cohort is not a permanent competitive restriction. Own viewing and moderation complete together before integrations. No offers before consumers/measured budgets qualify. Rollback disables admission/offers, revokes grants and discards queues; forward-only compatible schemas remain, gameplay continues, no legacy SPECTATE1 downgrade.

**Later packages:** relay extraction and Discord/Twitch/OBS follow native qualification. This plan assigns them no shared paths, IDs or migrations. They preserve consent/filtering, chosen D including 0, read-only gameplay semantics, account limits and audit. Any integration storage change is separately scoped, not automatically charged to an unused core reservation.

## 6. Owner decisions 2026-11-05

1. **Competitive casting:** allowed in PvP, wars, arenas and boss encounters. Remove competitive refusal; keep ordinary identity/scope fencing and privacy.
2. **Delay:** no mandatory delay; default **0 s**, optional **0–300 s**, applying to everything viewers receive as cast content. Rationale: OBS/Discord streams can already have no delay. Caster chat is live; interruption/revocation contains no fresh content.
3. **Viewer identity/access:** logged-in Platform accounts without a character; optional caster password/invite; **50 viewers/cast**, **2 viewer sessions/account**.
4. **Bystanders:** only after enabling **“allow me to be shown in casts”**, default **off**. Visible nonconsenting player means neutral slate. Private UI is never streamed.
5. **Live chat/moderation:** viewer chat shown **live in the caster's game client**, as in Tibia cast. Caster and authorized moderators can mute/kick. No viewer gameplay commands; cast chat is an isolated data/display channel.
6. **Priority:** own viewing/moderation first, Discord/Twitch/OBS later. In-node option A first within measured limits; relay B available for scale/isolation.
