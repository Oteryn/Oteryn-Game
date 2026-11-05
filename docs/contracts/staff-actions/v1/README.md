# S12-GM-CONTRACT v1 — wspólny projekt Game / Platform

## Pytania do właściciela
1. Czy zatwierdzamy wspólnie profil `staff.*`? Zalecenie: tak, po akceptacji obu zespołów i dodaniu operacji do katalogu ADM.
2. Czy etykiety tutor/gm/god wymagają migracji rosteru? Zalecenie: jawna migracja bez automatycznego podnoszenia uprawnień.
3. Czy wymagamy osobnych kluczy dla środowisk? Zalecenie: tak; zablokować wdrożenie bez rozłącznych list zaufania.

Zaakceptowane: 10 akcji/minutę/aktor, 100/minutę/środowisko oraz 30 sekund
dopuszczalnej nieaktualności danych przy niedostępności Platform.

## Authority and binding
Joint design pending Game and Platform acceptance. Normative base:
[ADR-0023](../../../architecture/ADR-0023-admin-control-plane-and-analytics-console.md),
[ADM-01](../../../architecture/ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md) and
[admin-control/v1](../../admin-control/v1/README.md).
This profile proposes new operation names; it does not advertise or enable them.
Existing `sanction.*`/enforcement contracts must be mapped jointly before rollout.

One staff action is one ADM request: `kind=command`, `operation_version=1`,
operation `staff.ban`, `staff.unban`, `staff.namelock`, `staff.warning` or
`staff.notation`. `action.schema.json` composes the ADM request schema with
this profile. The closed `params` contains only `sanction_id`, `expires_at`,
`sanction_revision`, `evidence_refs`, plus the ADM R4 control `preview_digest` (only for R4 with `dry_run` false).
The latter is required for executing R4, as required by ADM-01 §5; it is not
another staff identity or audit field. No duplicate actor, target, reason,
operation ID, timestamp or environment appears in params.

| Meaning | Single signed source |
| --- | --- |
| Command identity/idempotency | `admin_operation_id` |
| Request identity and replay | `request_id`, `nonce`, ADM 24-hour nonce protection |
| Actor and authorization | `actor.admin_id`, `roles`, `permission`, `mfa_at`, `step_up_at` |
| Second administrator | `approvals[]` |
| Request lifetime | envelope `issued_at` / `expires_at` |
| Target | `target.account_ref` (Platform account UUID string), `target.character_id` |
| Reason | `reason.code`, `reason.text`, `reason.ticket_ref` |

ADM-01 §3 makes `admin_operation_id`, not `request_id`, the command idempotency
key. Retries use fresh nonce/request evidence under ADM while preserving command
identity and canonical intent; conflicting params return ADM_IDEMPOTENCY_CONFLICT.
Do not bypass ADM verification order, JCS/Ed25519 signatures or key retention.

The ADM envelope signs `signer_key_id`, but the current ADM contract does not
explicitly mandate environment-scoped keys/trust lists. Therefore environment
binding is NOT guaranteed by the envelope alone. No environment field is added
to params: deployment must enforce disjoint trusted signer keys per environment,
validate the serving agent/target route and reject cross-environment signers.
This is a rollout prerequisite, not an assertion that signer naming ensures it.

## Ownership, permissions and controls
Platform owns account bans, namelocks, appeals, sanction state/revisions and the
existing `audit_log` / ADM-P2C evidence. Game owns world enforcement, all-session
kick on ban, character jail/teleport/notation and live reports. Game adds no
separate audit store. Existing ADM `admin_command_ledger` remains required by
ADM-01 §9 for durable effect/idempotency evidence; it is not Platform audit truth.

| Operation | actor.permission | Minimum risk |
| --- | --- | --- |
| staff.warning / staff.notation | game.sanction.write | R1 |
| staff.namelock | game.sanction.ban | R2 |
| staff.ban temporary / staff.unban | game.sanction.ban | R3 |
| staff.ban permanent | game.sanction.ban | R4 |
| World actions (existing separate operations) | game.player.moderate | ADM operation policy |

Reuse `game.sanction.read` for sanction inspection. Add no scopes.
Tutor, gm and god are roster product labels only: no automatic uplift and no
tier field in params. Enforce permission server-side, including revocation.
R3/R4 require step-up within five minutes of issuance; R4 requires prior dry run
and matching canonical preview digest. ADM-P2B enforces a second admin approval
of identical canonical intent, never self-approval; requester plus second admin
satisfy the two-person permanent-ban rule. Game independently checks ADM evidence,
approval freshness and preview match. Schemas cannot verify identity, approval
intent, signature validity, time freshness or semantic target ownership.

## Params and enforcement
`sanction_id` identifies the sanction being set or lifted. Ban/namelock require
params `expires_at`: UTC RFC3339 for temporary, null for permanent. This is
sanction expiry, separate from envelope request expiry. Other actions omit it.
Temporary expiry must follow decision time. `sanction_revision` is a monotonic
per-account Platform revision, within ADM safe-integer bounds, never session
generation. Platform allocates revisions transactionally. `evidence_refs` is a
bounded list of opaque Platform evidence references, never private player text.
Namelock targets a character (`target.character_id` required); notation may target account or character. Warning
is player-visible; notation is staff-only. Platform approves replacement names.

Platform commits decision, audit and delivery evidence atomically before dispatch.
Push uses existing ADM control delivery. Game hello resumes the durable delivery
watermark; reconcile on startup, reconnect and at least every 30 seconds using
the same control channel. Delivery watermark and per-account sanction_revision
are distinct. Gaps halt acknowledgement pending replay. Expired cursor requires
a consistent Platform snapshot/watermark. Snapshot/replay framing must be agreed
in ADM-P2C before enabling these commands; do not invent a separate endpoint.

Game installs the account revision barrier before disconnecting every live session
on every owning node with StaffKick/PLATFORM_SECURITY. Offline accounts still get
a barrier. Admission/login/resume/channel transfer check current Platform ban
state and the barrier under existing session-generation fencing. Older revisions
cannot undo newer state; identical revision with different intent is rejected.
Partial node failure remains pending, retried, never reported as applied.
Unban permits fresh admission, never resurrects sessions. Namelock disconnects
and blocks affected characters until lift/approved rename. Warning delivery is
idempotent, deferred to admission when offline; notation has no player effect.
World actions retain separate WorldId/ChannelId and session-generation fences.

After 30 seconds without fresh authoritative state, refuse new login/resume/
transfer; preserve existing sessions and enforce known restrictions. Expiry uses
Platform authoritative state; delayed messages cannot revive an expired ban.
Persist consumer receipts/cursors in existing ADM durability. Crash after effects
before results must recover without duplicate warning or repeated world effects.

## Results, audit and abuse prevention
`result.schema.json` describes only the operation-specific ADM `result` payload:
`sanction_id`, `sanction_revision`, `enforcement`, optional `evidence_refs`.
Request/command correlation, status, timestamps, signature, effective_revision,
audit_event_ids and game_ledger_id belong solely to the ADM result envelope.
Map pending to accepted/in-progress and applied to final success; errors use ADM
error/status fields. There is no `rejected` enforcement value: a refused, stale or
superseded command (for example an older `sanction_revision`) is reported only
through ADM error/status, never as a successful result. Carry the result as a signed existing ADM results envelope
following hello, never a separate ban-applied endpoint. The current closed ADM
OperationResult union must be extended jointly before advertising staff commands.

Every accepted, denied, failed, rate-limited and retried action is evidenced in
Platform audit_log and ADM-P2C with actor, approvals, reason, target, revisions,
canonical request digest, signature/key reference and signed Game results.
Use existing retention/access rules. Existing ADM ledger/effect transaction and
result retries preserve evidence across crashes; never claim a persisted audit
row when storage failed. Platform audit failure blocks dispatch.
Enforce token buckets of 10/minute/actor and 100/minute/environment before effects,
including bulk submissions; replay retries do not repeat effects. No emergency
or permanent-ban approval bypass. Recheck approvals after any intent edit.

## Phased implementation and tests
1. Joint: accept profile, register staff operations and risk/permission policies,
   extend ADM result union/catalog, confirm environment trust separation and
   ADM-P2C snapshot/replay. Test signatures, replay, conflicting idempotency,
   cross-environment keys, R4 preview mismatch and self/stale approval rejection.
2. Platform: `portal-rs/crates/admin/src/control/` (store, signer, canonical,
   controls, http), `portal-rs/crates/admin/src/console/{approvals,audit}.rs`,
   `portal-rs/crates/support/src/sanctions/`,
   `portal-rs/crates/admin/src/rbac/catalog.rs`. Implement revision allocation,
   audit/outbox, approvals/appeals, reconciliation and limits. Test transactional
   rollback, expiry, permission revocation, identical-intent approval and replay.
3. Game server: `crates/gameplay/social-rules/src/staff.rs`,
   `crates/admin/{contracts,agent}/`, `apps/game-server/` admission/control paths.
   Test concurrent admission/ban, multi-node pending, restart after effect before
   acknowledgement, offline ban, revision reordering, snapshot gap and 30-second
   staleness; preserve existing session-generation fences.
4. Native panel: `crates/client-ui/src/features/0051-staff/`. Replace unsupported
   handlers with capability-gated inspect/reports and typed actions, reason,
   duration, approval/pending/error display. Test revoked permissions, duplicate
   submit, old clients and evidence privacy. Enable after shared server acceptance.

## Golden vectors and validation
Five golden files are full ADM structural request fixtures, not valid signed
commands: signature and R4 preview digest are explicit test placeholders. Validate
each against action.schema.json and admin-control/v1/request.schema.json with
Draft 2020-12 and format checks. Cryptographic executable vectors belong to the
ADM agent suite when implementations land; schema acceptance is not authorization.
Validate result.schema.json as a schema; its ADM union integration is phase 1.
