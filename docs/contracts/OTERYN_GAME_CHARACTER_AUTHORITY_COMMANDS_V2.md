# Character Authority services v2 (G-3)

Status: **implemented against Platform PROPOSED revision 2 @a85b5da;
activation pending**. This records the Game implementation and the resolved
LCFA interpretation below; it does not activate services.

Owner implementation authorization: 2026-11-13. Source:
[Platform proposal at the pinned revision](https://github.com/Oteryn/oteryn-platform/blob/a85b5da4de23f3e8633cfb25a2db1ad7f6133acd/docs/contracts/OTERYN_V2_CHARACTER_SERVICES_COMMAND_CONTRACT_PROPOSAL.md).
The proposal's pending acceptance status is retained.

## Wire implementation

`crates/gameplay/native-admission-source/src/character_services_v2.rs` defines
separate validated types for list request/response, read request, intent,
receipt, private service snapshot, watermark and acknowledgements. Constructors
and decoders use the same closed validation. The decoder retains member order
and rejects noncanonical re-encoding, duplicates, unknown/missing members,
escapes, trailing whitespace, invalid UUIDv7/decimal strings, wrong variants,
invalid nullable-state combinations and oversize bodies. Wire names use NFC
and Unicode general categories Cc/Cf; requested names use CHAR-NAME-1 ASCII.
The already-locked Unicode implementations are reused, without a new version.

Bounds are 512/8192 for list, 512/4096 for read, 1024/256 for receipt,
2048/256 for state, 512/256 for watermark. The paths are the proposal's five
compiled `/internal/v2/game-auth/` paths. No listener is added. Command calls reuse the existing command mTLS identity,
HTTP/1.1/TLS 1.3 transport and deadline/capacity conventions. V2 responses
reject chunked framing, non-identity encoding and nonempty failures, and
require no-store. Dedicated service-state delivery is described below.

`MAX_INTENT_TTL_SECONDS` is 300 and `MAX_FUTURE_SKEW_SECONDS` is 5.
`CharacterCommandIntentV2::valid_at` requires a trusted clock and exclusive
expiry. A deletion deadline need only be strictly later than scheduling on the
wire; 60 days must not be enforced as a wire invariant.

The twelve §9 golden blocks are copied byte-for-byte, without a terminal
newline, into `fixtures/character-services-v2/`. Nine are valid; three are
intentionally invalid. Other canonical request/response examples are tested
inline. V1 codecs and decisions are unchanged; their fixture assertions are
not modified.

## Decisions and storage

Migrations 2830–2833 prepare repeatable receipts, lifecycle/deletion state,
released-name holds, and the independent service-state outbox, respectively.
0087 remains unchanged. Storage for the companion is installed with decisions
so mutations cannot commit without their services work; its publisher is a
separate delivery part. Full original intent and receipt bytes are immutable.
The existing issuer-wide operation advisory lock serializes all receipt writers.

`DELETION_GRACE_SECONDS` is 60 days; `RELEASED_NAME_HOLD_SECONDS` is 30 days.
The portfolio lock precedes the root/CharacterLease locks. Cancel and finalizer
recheck the same durable deletion under those locks. Pending and deleted roots
cannot acquire a fresh session or ordinary gameplay revision. Finalization
retains the CharacterId and gameplay references, emits a DELETED snapshot and
omits the character from LCFA; it does not physically erase gameplay history.
The old comparison key remains reserved for everyone until its hold expires.

Service changes carry immutable revision-chain and audit evidence, preserving
all existing progression metadata. A missing initialized progression state is
an unavailable dependency, never fabricated progression. Account security,
current node incarnation, restore fence, ownership, revision and offline state
are checked in the deciding transaction. Unsafe/unavailable or backward-moving
service time commits nothing; finalization waits. The service clock reuses
the existing Character Authority/account-closure trusted server clock. A null or
backwards source time is unavailable; no new production clock assurance value is
selected. Production clock assurance remains a deployment prerequisite.

Refusals use the closed §5 codes. A same-comparison-key rename uses
`CHAR_CMD_NAME_UNAVAILABLE`: the closed vocabulary has no separate no-op code.
A mismatched deletion generation uses `CHAR_CMD_STALE_PRECONDITION`. A second
schedule uses `CHAR_CMD_LIFECYCLE_CONFLICT`. Reserved sex/world commands are
`CHAR_CMD_UNSUPPORTED` and cannot mutate state. Definite refusals store only
REJECTED facts; dependencies and ambiguous database outcomes store no refusal.
Exact replay publishes the original bytes; contradictory binding cannot overwrite it.

Detailed bindings are retained while financial/restore resolution is unknown;
no automatic premature GC is introduced. As required by §8, the configured
backup retention/restore horizon must be pinned before activation. No production
activation or certificate allocation is authorized.

## Consumer and configuration

`OTERYN_CHARACTER_COMMANDS_V2_ENABLED` is false when absent. Only exact `true`
and `false` are accepted. It overrides the default-false Game setting
`platform.services_v2_enabled`; enabling requires the configured existing
character-command identity. Neither setting is enabled by this change.

The existing consumer alternates its v1 and v2 routes on one command source,
with independent cursors and the existing key retry, receipt quarantine,
short-page rescan, eight-page/30-second rescan and 1–10-second backoff rules.
V1 encoders and successful fixtures are unchanged. With v2 off, its source
performs only the original v1 requests. The existing command worker drives
the new deciding entry and a bounded automatic-finalization pass; finalization
is attempted even when the Platform read is unavailable. Account security is
refreshed through the existing held-evidence path immediately before deciding.

Both profiles use the issuer-wide operation advisory lock. Legacy creation,
bootstrap, Bazaar and services detect an operation already decided by another
profile before attempting a mutation. The shared 40-operation issuance backlog
and 256 pending ceiling remain obligations of the one Platform issuer, not two
separate version-specific issuance capacities. Game issues no operations and
uses one serialized consumer/deciding writer; reading a key never authorizes it.
Receipts refused with 404/409 remain durable and automatic publication stops.

## Companion delivery and resync

`[platform.character_services]` supplies its dedicated certificate, private-key
file, source authority, and the **same** `epoch_fence_file` as LCFA. Enabling v2
requires both sections. Certificate public keys are compared against all other
configured internal purposes, including command, LCFA, runtime/evidence,
bootstrap, Store/Coins' existing identity, public/T94, journal, Premium and stream
identities. The gameplay identity is also refused. Paths alone are insufficient
for identity separation. No certificate, key or production setting is supplied.

The command worker owns a separate serialized services publisher. It shares
LCFA's transport, watermark/idle constants, transaction-slack computation and
bounded phase helper. The existing LCFA publisher remains the sole writer of
the shared external epoch witness. Services only publish when the DB epoch
matches that durable witness; they never raise an epoch or rewrite its file.
Every store pass checks recovery and current process authority. The receipt's
projection revision is the independent services counter.

A mutation stores a coalesced complete `(account_id,character_id)` snapshot.
Each account has an independent services counter; ACK matches exact epoch and
revision and cannot clear newer work or a quarantine. LCFA ACKs do not touch
services, and services ACKs do not touch LCFA. An unacknowledged coalesced row
keeps its oldest committed time, so replacing it cannot falsely advance
completeness. Lost ACKs retry the same assigned facts/bytes. Source time is
assignment time, not retry time.

A never-enabled authority does not capture ordinary v1 changes. Capture starts
atomically with its first service mutation or with the enabled publisher, and
continues durably across publication pauses. This preserves already-created
state for recovery. Initial enable and epoch raises queue bounded resync:
sixteen subjects per pass, roots followed by retained old-owner removals,
including DELETED tombstones. The watermark stays below resync start until
enumeration completes. Late rows from the previous epoch are repaired under the
same portfolio-before-root order. Resync does not change LCFA revisions.

`REMOVED_FROM_ACCOUNT` uses retained old-owner facts, never the new owner's
name, sex or subsequent revisions. Invalid snapshots and 400/401/409/413 replies
quarantine their scope durably; current-generation updates do not erase that
disposition. A watermark conflict quarantines the feed. Work remains pending and
holds completeness back. Repair requires operator investigation and an approved
shared epoch resync. Receipt publication stops are separately retained in 2830,
bound to immutable receipt bytes and recovery generation, including restart.

## Conservative readings and activation prerequisites

The proposal says, verbatim:

> backoff on 429/503 at most 10 s

and:

> 429 = throttled with Retry-After in seconds

The publisher's own exponential backoff is bounded at ten seconds. A larger
server Retry-After is an independent embargo and is never shortened to satisfy
that local cap. It can suspend the nominal watermark cadence: the feed becomes
stale/unavailable, rather than ignoring the peer's embargo or fabricating
freshness. This operational conflict needs exact-peer confirmation before
activation.

The proposal also says:

> A missing authoritative dependency, unknown eligibility, unsafe clock, unreconciled restore, database deadlock/serialization failure or loss of process proof commits nothing and stores no receipt; Game retries the same operation.

An absent initialized progression baseline therefore keeps a service pending;
this path does not fabricate gameplay initialization metadata or an admitted
session to obtain it. Existing gameplay initialization must establish that
owning dependency first. Same-key contradictory heads remain conflicts even
when another malformed member prevents full decoding.

For retention the proposal says:

> Terminal detailed service payloads/old names/deletion timestamps are purged 90 days after finalization of the operation **and** resolution of associated financial/restore disputes, whichever is later.

It supplies no Game read or acknowledgement of financial-dispute resolution.
Unresolved bindings are consequently retained; no premature erasure or
anti-replay GC is inferred from elapsed time. The configured backup-retention
horizon must also be pinned before activation, as §8/Q7 require. Legal erasure
or a paused grace window requires operator recovery review. No deployment,
production enablement, PKI allocation or joint Platform E2E is implied.

## Resolved interpretation, pending Platform wording confirmation

The proposal §6 says, verbatim:

> service changes must not advance LCFA v1 revisions

The accepted Game LCFA contract §5 says, verbatim:

> **`projection_revision`** is per `AccountId`, monotonic, and advanced in the same Character Authority transaction as every mutation that changes the account's list or a listed field: create or bootstrap, rename, lifecycle change, world transfer, and ownership transfer (both the old and the new owner's accounts).

The accepted LCFA field table describes `name` as:

> current display name

It also says:

> Terminally deleted, retired or finalized Characters are omitted.

Coordinator decision, 2026-11-13: the accepted LCFA v1 contract wins because
this proposal says "LCFA v1 is unchanged". Only changes exclusively to services
facts (deletion operation/deadline, sex, CharacterRevision, unlisted lifecycle
detail) leave LCFA revisions unchanged. Every change to an LCFA-listed field
or its character set advances LCFA in the same transaction through the existing
outbox path. Rename advances both feeds; schedule/cancel/finalize change LCFA
availability or membership and also advance both. Services revisions remain an
independent per-account namespace under the shared epoch. Receipt
`projection_revision` always means the services revision. Each feed has its own
acknowledgement. A services-only change queues only services work. Platform is
asked to confirm this wording; implementation does not wait for confirmation.

## Projection isolation and conservative lifecycle handling

Projection capture uses wall-clock seconds clamped to the account's previous
observation, independently of the strict service-decision clock. Ordinary
progression writes cannot fail on service clock unavailability or a backward
clock step. Initial migration and epoch changes queue bounded resync work;
they do not synchronously walk all roots. JSONB is internal facts storage:
the publisher reconstructs the specified member order through the production
canonical codec before sending any bytes. A Bazaar-locked lifecycle has no
matching lifecycle in this proposal; its scope remains unavailable rather
than advertising ACTIVE. A later eligible root change queues replacement work;
publication quarantine still requires an approved epoch/resync repair.


## Review repairs

Terminal lifecycle-4 roots remain immutable history but do not occupy portfolio
slots or enter a new CloseAccount manifest. The existing account-wide closure
fence still covers their account; lifecycle 4 independently denies admission.
Bazaar capacity uses the same live-root view. Receipt replay checks the original
binding, process and recovery fences before any account-security refresh.
Oversized reads reuse v1's bounded oversized-command binding and store a terminal
invalid-intent receipt; a corrected intent cannot reuse that operation.

Resync rereads ownership under the portfolio and root locks and preserves
same-epoch quarantine. Phase one pages by the existing CharacterId primary key,
then locks the bounded batch in account order: no new roots index is built.
Migration 2831 adds the widened lifecycle CHECK NOT VALID; migration 2832
validates it in a separate migration transaction after the ACCESS EXCLUSIVE
DDL lock is released. Validation uses SHARE UPDATE EXCLUSIVE and permits ordinary
row writers. This follows the repository's NOT VALID convention without holding
the addition lock throughout the validation scan. No concurrent-index operation
is attempted inside the migration transaction.

Finalization skips candidates with a session/presence/lease holder and rechecks
that state under its deciding locks. V1 and v2 retain one exchange in flight,
with separate retry counters and next-poll deadlines; a v2 backoff never sleeps
past a due v1 poll.
