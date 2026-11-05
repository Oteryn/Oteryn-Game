# S12-GM-CONTRACT v1 — projekt wspólny Game / Platform

## Pytania do właściciela (do wspólnej akceptacji)
1. Czy zatwierdzamy podział odpowiedzialności poniżej? Zalecenie: tak; Platform pozostaje źródłem prawdy o koncie.
2. Czy tutor/gm/god zastępują obecne role? Zalecenie: tylko nazwy uprawnień produktu; jawna migracja rosteru, bez automatycznego podnoszenia praw.
3. Jakie limity operacyjne? Zalecenie: 10 zapisów/minutę/aktor, 100/minutę/środowisko; stałe bany zawsze wymagają dwóch osób.
4. Jak zachować się przy utracie Platform? Zalecenie: blokować nowe logowania po 30 sekundach nieaktualności; istniejące sesje zachować, egzekwować znane sankcje.
5. Kiedy zamknąć powiązanie ADM? Zalecenie: przed implementacją wspólna akceptacja dokładnych referencji schematów i wektorów podpisów przez Game i Platform.

## Status and authority
Design proposal, not production authorization. Both Game and Platform must approve v1.
Base is current origin/main. This checkout lacks ADR-0023 and
`docs/contracts/admin-control/v1/`; exact envelope fields, canonicalization,
key rotation and signature vectors therefore remain a joint integration gate.
Do not introduce a parallel envelope, endpoint, signing algorithm or scope.
The schemas here describe payloads only; golden vectors are unsigned payload
examples, not cryptographic interoperability evidence. Wrap them in the existing
ADM hello/results control channel and its authenticated command delivery.
Reuse ADM-01 §8 scopes exactly as supplied by Platform.

Current server supports Inspect/Mute/Unmute/Kick in
`crates/gameplay/social-rules/src/staff.rs`; native handlers in
`crates/client-ui/src/features/0051-staff/registrations/handlers.json` are unsupported.
The live-readiness review §208 assigns bans to Platform (P1).
Its MODERATOR/GAMEMASTER proposal and current GM/SENIOR_GM/ADMIN roster
must not be silently equated with the proposed Tibia-style tiers below.

## Ownership and permissions
Platform owns account bans, namelocks, appeals, sanction revisions, identity,
`audit_log` and ADM-P2C evidence. Game owns in-world enforcement, all-session
kick on ban, character jail/teleport/notation and live report handling.
Game emits evidence; it owns no separate audit store. Existing staff audit
records remain existing evidence, not a new competing sanction authority.

| Product tier | Allowed work | Required existing scope |
| --- | --- | --- |
| tutor | inspect limited sanctions, triage reports | game.sanction.read |
| gm | warning/notation, mute/unmute/kick, jail/teleport | game.sanction.write for sanction writes; game.player.moderate for world actions |
| god | gm plus account ban/unban/namelock proposals | game.sanction.ban plus game.sanction.write |

Role and scope are both required, checked server-side for every request.
Read access does not expose private evidence or account identity beyond need.
No client can grant roles. Platform rechecks authority at commit, including
revocation; Game rechecks local world action authority and session generation.
Appeals and lifts are Platform decisions, never client-side deletion.

## Payload families
`action.schema.json` defines ban, unban, namelock, warning and notation.
IDs are opaque nonempty strings; all timestamps are UTC RFC3339. Each action
has an operation ID, idempotency key, environment, actor ID and product tier,
account ID, optional character ID, monotonically increasing account security
generation, reason code, bounded explanation and evidence references.
Ban and namelock require an explicit expiry (temporary) or null (permanent).
Unban references the lifted sanction; warning and notation have no expiry.
Namelock blocks character admission until Platform approves a replacement name;
it is not a Game-side rename. Warning is player-visible; notation is staff-only.
Character notation requires character ID. No sensitive evidence enters player text.
All IDs, scope, actor, target, environment, generation, expiry and approvals must
be covered by the ADM signature. Reject unsigned, expired, wrong-environment,
unknown-key or invalid-signature envelopes before effects. Use ADM replay and
key-retention rules; retain verification material with ADM-P2C evidence.
Permanent bans require two distinct authorized human approvers, neither a
service identity; Platform verifies their recorded approvals of identical bytes.
Schemas cannot establish authority, timestamp ordering or signature validity.

## Delivery, replay and enforcement
Platform commits sanction state and audit/outbox evidence atomically, then pushes
ordered updates through ADM. Game hello carries the existing ADM resume cursor;
reconnect replays from the last durably acknowledged contiguous watermark.
Watermark means ordered delivery sequence, not account security generation.
Gaps stop acknowledgement and trigger replay. An expired cursor requires a
Platform snapshot and its consistent watermark before incremental consumption.
Pull reconciliation over the existing ADM control channel is required on startup
and reconnect and every 30 seconds; no new public reconciliation endpoint.
Exact payload binding to ADM remains gated as above.

Game verifies admission against fresh Platform account state, rejects banned
accounts on login/resume/channel transfer, and checks generation under the
admission/session fence. On ban, install the account generation barrier before
ending every live session on every node with StaffKick/PLATFORM_SECURITY.
A concurrent admission cannot escape the barrier. Offline accounts still get
a barrier. Older generations cannot undo newer state; expiry uses authoritative
Platform time/state and never makes an older delayed ban active again.
Unban permits fresh admission only; never restores a disconnected session.
Namelock disconnects affected characters and refuses their admission.
Warning delivers once when online; offline delivery waits for admission.
Notation has no player-visible effect. World actions use WorldId and ChannelId
separately and retain existing session-generation fencing.

Deduplicate by environment + idempotency key with a canonical payload digest:
identical retries return the original result, conflicting reuse is rejected.
Persist enforcement receipt/cursor in existing ADM consumer durability, not a
new audit database; acknowledge only after barrier/effects are durable.
A crash between effect and result retries safely. Report partial node failures
as pending, retry, and never claim applied until all owning nodes are fenced.
Signed `result.schema.json` payloads travel ONLY in existing ADM results
following hello, correlated to the command. Platform stores them in audit_log
and ADM-P2C evidence. There is no separate ban-applied endpoint.

## Audit and abuse controls
Every accepted, denied, rate-limited, failed and retried action is correlated in
Platform audit_log with actor, approvers, reason, target, before/after revision,
request digest, signature/key reference, timestamps and signed Game result.
Use existing ADM evidence retention/access rules; redact evidence from clients.
Do not acknowledge unaudited effects: Platform audit/outbox commit precedes
command dispatch; Game results are durably retried until Platform acknowledgement.
Apply token buckets before effects: recommended limits above; no batch bypass.
Permanent-ban approvals are distinct and bound to the exact request digest.
Prevent self-approval and stale approvals after edits; audit emergency actions too.

## Phased implementation and acceptance
1. Joint Game/Platform: pin ADM-01/ADR-0023 versions, bind payloads to existing
   schemas, approve role mapping and durations; validate shared signed golden
   vectors, wrong environment/key/signature, replay and conflicting idempotency.
2. Platform (owned paths to confirm in Platform repository): existing ADM control
   producer, account sanction service, audit_log and ADM-P2C evidence; atomic
   sanction/outbox writes, approvals, appeals and snapshot/replay. Test approval
   revocation, expiry, duplicate delivery and audit failure rollback.
3. Game server: `crates/gameplay/social-rules/src/staff.rs` and
   `apps/game-server/` existing admission/control consumers. Add generation
   barriers and signed ADM results. Test concurrent admission/ban, multi-node
   partial failure, restart after effect before acknowledgement, gap/snapshot,
   stale generation, offline account and login/resume refusal.
4. Game native panel: `crates/client-ui/src/features/0051-staff/`;
   capability-gated report queue, inspect, typed actions, durations/reasons,
   approval status and correlated results. Test unauthorized/stale roles,
   unsupported old clients, duplicate submit, pending/failure presentation and
   absence of private evidence. Enable only after both owners accept phase 1.

## Validation
Validate both Draft 2020-12 schemas and all five payload vectors; additionally
run repository contract registry validation. Negative runtime cases above are
implementation acceptance requirements, not claims of implemented behavior.
