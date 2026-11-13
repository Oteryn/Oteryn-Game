# Character Authority services v2 (G-3)

Status: **wire implemented against Platform PROPOSED revision 2 @a85b5da;
activation pending**. Decisions, storage, consumers and delivery remain blocked
on the projection contract conflict below. This is not service activation.

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

## Planned decisions and storage — not implemented

The approved Game policies are a 60-day deletion period and a 30-day
released-comparison-key hold, including the former owner. Cancellation must be
offline and strictly before the deadline; unsafe time prevents cancellation
and delays finalization. These decision policies are not executable in this
wire-only slice.

No migration is used here. 0087 is unchanged. The assigned future slots remain
2830 for repeatable receipts, 2831 for deletion/lifecycle, 2832 for name holds,
and 2833 for an independently acknowledged service-state outbox. Both command
versions must share issuer-wide operation identity and one deciding writer.
Full canonical intent bytes, not a fingerprint alone, must bind replay.

The wire validator implements the closed §5 refusal set and command-specific
applicability. `CHAR_CMD_OK` is committed-only. REJECTED has no result facts.
Reserved sex/world shapes decode, but no execution is implemented; their
future decision is `CHAR_CMD_UNSUPPORTED`. The in-slice decisions must select
§5 codes under the owning transaction; this codec makes no eligibility decision.

No v2 puller, publisher, flag activation, retention job or migration is present.
No payload or identifier is logged. No production enablement is authorized.

## Blocking contract conflict

The proposal §6 says, verbatim:

> service changes must not advance LCFA v1 revisions

The accepted Game LCFA contract §5 says, verbatim:

> **`projection_revision`** is per `AccountId`, monotonic, and advanced in the same Character Authority transaction as every mutation that changes the account's list or a listed field: create or bootstrap, rename, lifecycle change, world transfer, and ownership transfer (both the old and the new owner's accounts).

The accepted LCFA field table describes `name` as:

> current display name

It also says:

> Terminally deleted, retired or finalized Characters are omitted.

The current snapshot query reads `name` and `lifecycle` directly from current
Character roots. Therefore a rename or finalization changes LCFA content. Not
advancing its revision permits equal-key/different-content publication; hiding
that change in a historical LCFA snapshot instead changes the accepted current
snapshot and completeness semantics. The proposal does not specify which LCFA
requirement is amended. No such amendment is inferred here.

Before parts B–D, the owner must specify the LCFA disposition for these mutations
and the corresponding completeness/resync rules. Decisions cannot safely be
wired to either projection until that conflict is resolved. The independent
services revision/ack requirement itself is clear and is retained.
