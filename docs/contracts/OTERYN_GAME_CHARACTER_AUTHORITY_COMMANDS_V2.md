# Character Authority services v2 (G-3)

Status: **wire implemented against Platform PROPOSED revision 2 @a85b5da;
activation pending**. Decisions, storage, consumers and delivery are being implemented under the
resolved interpretation below. This is not service activation.

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
compiled `/internal/v2/game-auth/` paths. No listener or HTTP caller is added.
Transport, peer binding and header validation belong to the consumer/publisher
parts, which are not implemented by this wire slice.

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
service time commits nothing; finalization waits. The service clock uses the
existing trusted server clock, refusing a statement delayed more than one second;
production clock assurance remains a deployment prerequisite.

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
