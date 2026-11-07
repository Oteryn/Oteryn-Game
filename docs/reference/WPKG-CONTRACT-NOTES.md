# Integrated producer contract notes

Package GPROD-PKG composes all three deliveries below on current main. The delivery
sections retain their original scope/history; statements about another producer being
absent describe that delivery's base, rather than this combined package.

The configured Bazaar and deletion producers and the default T94 section retain their
delivered ON settings. All supervision, authority and recovery paths remain composed.
The internal `PublishAccountClosureCommandReceiptV2` transport tag uses the existing
Character command certificate, receipt URL and wire operation; it selects deletion's
2048-byte request bound while CC v1 retains its 1024-byte bound. Bazaar's separate
receipt extension keeps its delivered bound. There are no new public wire codes.

The shared HTTP parser selects header requirements and retained retry-delay limits
independently: Store keeps its required header and 30-second limit, Bazaar retains
up to 30 seconds before its consumer applies the 10-second backoff cap, and T94
keeps optional headers and a 10-second limit. Pending deletion jobs reserve the same
Character operation namespace as transfers and creation. Bazaar/T94 client identities
and deletion/T94 epoch files remain separate when these producers run together.

## Delivery gbaz

# CONTRACT-NOTES — GPROD-BAZAAR

## Owner instruction and rollout

The latest owner instruction for this task supersedes the candidate contract's
original default-off rollout requirement: a configured Game Bazaar producer is
**enabled by default**. `platform.bazaar.enabled = false` remains an explicit
operational pause for the dedicated Bazaar command feed. Retained receipts,
custody and funding continue to reconcile; a pause never releases value.
`game_bazaar_configuration.enabled = false` is the durable authority pause across
both command feeds, including lock-bound CC v1 ownership transfers. Both switches
default true. Pause that durable gate when stopping all new authority effects;
the node switch alone does not disable the separately configured CC consumer.

An omitted `[platform.bazaar]` table means the deployment has not supplied the
required dedicated transport identity. It is not an implicit product feature-off
setting. Existing node configurations without Bazaar credentials remain parseable;
the shipped Synology node template now supplies the enabled Bazaar and Character
command sections. A deployment that selects those sections must provision valid
credentials before startup. No identity or private key is fabricated or silently
borrowed from another purpose.

## PLATFORM CONFIG

The Game-initiated connection uses existing `[platform]` settings:

| Key | Meaning |
|---|---|
| `endpoint` | Configured private Platform mTLS socket address; no request-selected endpoint |
| `peer_name` | Expected TLS server name and HTTP Host |
| `trust_roots_file` | Existing provisioned Platform CA roots |

The shipped `deploy/synology-game/node.toml.template` uses these purpose-specific
settings. `<BASE>` is the existing deployment substitution, not a runtime default
secret location.

```toml
[platform.character_commands]
client_certificate_file = "<BASE>/node/secrets/character-commands.crt"
client_key_file = "<BASE>/node/secrets/character-commands.key"
source_authority = "oteryn:character-authority:primary"

[platform.bazaar]
enabled = true
client_certificate_file = "<BASE>/node/secrets/bazaar.crt"
client_key_file = "<BASE>/node/secrets/bazaar.key"
source_authority = "oteryn:game:primary"
```

`enabled` defaults to `true` when omitted from a configured Bazaar table.
Certificate/key paths and `source_authority` are required values. Paths must be
absolute and purpose-separated. Certificates are checked for shared public keys
against the evidence, Character receipt, runtime-status and projection identities;
changing a path cannot evade that check. Existing private-file ownership and mode
checks apply. Platform must independently authorize the dedicated Bazaar peer for
its six routes, and the Character peer for the existing receipt route.

Bazaar `source_authority` is the Game projection/snapshot namespace. The durable
snapshot policy's `snapshot_source_authority` must equal it. Receipt
`source_authority` remains the configured Character command namespace. Neither
field selects trust. The actual provisioned namespaces may differ from the
synthetic fixture examples, provided both sides configure the same bindings.

The authentication names are purpose/authority identifiers, not configurable
endpoints or credentials:

| Binding | Exact identifier |
|---|---|
| Dedicated Game Bazaar mTLS descriptor purpose | `OTERYN_GAME_BAZAAR` |
| Existing Game Character command/receipt mTLS descriptor purpose | `OTERYN_GAME_CHARACTER_COMMANDS` |
| Bazaar intent issuer | `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| Bazaar intent audience and snapshot issuer | `OTERYN_GAME_BAZAAR_AUTHORITY` |
| Snapshot audience | `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| Existing CC v1 transfer intent issuer | `OTERYN_PLATFORM_CHARACTER_AUTHORITY` |
| Existing CC v1 transfer intent audience | `OTERYN_GAME_CHARACTER_AUTHORITY` |

The pending-command feed and operation-state comparison feed are the HTTP
operations named below, not broker topics. The contract allocates no new named
broker/Gateway topic. Its registered audit family remains
`BAZAAR_AUTHORITY_DECISION`, event type `6`, schema `1`, in the existing ANL
`EventEnvelope`/`TransactionEventRef` facility. Deployment authorizes the actual
provisioned mTLS certificate subjects for these purposes; these identifier strings
never select a certificate, public key or peer allowlist from request data.

All methods below are POST; Game initiates every connection. There is no inbound
Game listener or Bazaar receipt endpoint.

| Operation | Compiled Platform path | Request / response cap |
|---|---|---|
| `ListPendingBazaarCommandsV1` | `/internal/v1/bazaar/commands/pending` | 512 / 8192 bytes |
| `ReadBazaarCommandIntentV1` | `/internal/v1/bazaar/commands/read` | 512 / 4096 bytes |
| `ListBazaarCommandStatesV1` | `/internal/v1/bazaar/commands/reconcile` | 512 / 12288 bytes |
| `PublishBazaarTransferabilityV1` | `/internal/v1/bazaar/transferability` | 262144 / 256 bytes |
| `PublishBazaarListingStatusV1` | `/internal/v1/bazaar/status` | 2048 / 256 bytes |
| `PublishBazaarWatermarkV1` | `/internal/v1/bazaar/watermark` | 512 / 256 bytes |
| `PublishCharacterCommandReceiptV1` (CC v2 body) | `/internal/v1/game-auth/character-commands/receipt` | 270336 / 256 bytes |

The larger CC v2 receipt cap is selected by an internal transport operation tag;
it changes neither the wire operation name nor the endpoint or identity. Existing
CC v1 publication retains its 1024-byte request cap and bytes.

CC receipt acknowledgements remain exactly
`{"contract_version":1,"result":"accepted"}` as CC §4.3 requires. The Bazaar
proposal explicitly preserves that acknowledgement. A v2 receipt body does not
invent a v2 receipt acknowledgement. Bazaar projection/watermark acknowledgements
are exactly v2 `accepted` or `superseded`; command receipts never accept
`superseded`.

Connections use TLS 1.3 and HTTP/1.1. Connect deadline is 1 second, handshake 2
seconds, command exchange 3 seconds, and ingestion 5 seconds. Both discovery and
reconciliation cursors rescan after at most eight full pages or 30 seconds. One
exchange is in flight per consumer. Poll cadence is 1 second and retry backoff is
bounded at 10 seconds. The HTTP parser preserves a valid positive `Retry-After`;
the Bazaar consumer applies the contract's 10-second ceiling. All responses
require `Cache-Control: no-store`.

Receipt 404/409 retains the Game receipt and stops that key's automatic
publication with an alarm. Malformed/authentication/over-bound refusals require
repair and also retain evidence. Network errors, 429 and 503 retry the same bytes;
none supplies a terminal Game refusal. A lost acknowledgement replays stored
bytes. Watermarks come only from durable completeness/resync evidence, never from
an empty pending feed or an HTTP success.


### Authenticated intent-read conflicts

The candidate inherits CC §4.2/§5.1 operation-identity decisions, while the pinned
Bazaar proposal says an HTTP read failure does not prove that a Game effect is
absent. The implementation preserves both boundaries: an authenticated empty-body
read `409` becomes typed internal `Input::ReadConflict`, passed to the Game durable
authority. The transport itself never manufactures a receipt. Under current
process/recovery fences and operation serialization, Game first preserves any
existing immutable decision. Only an unseen operation can receive a new durable
`CHAR_CMD_OPERATION_CONFLICT` refusal, with no effect, so that attempt cannot
later commit. It does not release or reverse pre-existing asset custody or funds.

An oversized read `200` similarly becomes typed `Input::Oversized` and can produce
Game's own durable `CHAR_CMD_INTENT_INVALID` refusal after the same identity
checks. These technical inputs use separate internal binding tags; no
caller-supplied byte marker is interpreted as trusted transport evidence. A read
`404` leaves the attempt PENDING/unavailable and alarms; it never supplies
no-late-commit or asset-absence proof. Receipt-publication `404`/`409` continues to
retain the original Game receipt and stop automatic publication for that key.

## Qualification and reference identities

The identifiers, key IDs and public keys under
`docs/contracts/fixtures/bazaar/` are synthetic conformance inputs. They are not
production account allocations, peer subjects or signing trust. Provisioning real
mTLS and snapshot signing credentials remains a normal deployment dependency;
the implementation does not promote fixture keys into deployment trust.

The transport tests drive the actual Consumer with a deterministic exchange and
exercise exact schemas, closed routes, existing CC v1 cap preservation, explicit
pause/drain behavior, receipt-conflict retention, no-store responses and bounded
Retry-After handling. The mounted dispatch qualification invokes the same
production `dispatch_bazaar_intent` helper used by the node adapter after fresh
account-evidence refresh. PostgreSQL and full fixture execution evidence belongs
in the final task report; these notes make no separate test-pass claim.


### Durable producer policy defaults

No new environment variable is read. `OTERYN_TEST_POSTGRES_ADMIN_URL` is the
existing private test route, not a deployment setting. Deployment policy is the
single operator-owned row `game_bazaar_configuration`: `enabled` defaults **true**;
`policy` defaults `{}`, interpreted through the following typed defaults. Runtime
and Platform SQL identities cannot change that row.

| JSON policy key | Default |
|---|---|
| `obligations_revision` | `1` (the conservative implemented obligation matrix) |
| `cooldown_seconds` | `2592000` (30 days) |
| `snapshot_survives_transfer` | `true`, supported Character-owned snapshot profile only |
| `escrow_account_id` | `null`; a real Platform-issued AccountId must be configured |
| `escrow_capacity` | `64`, checked again when transferring into escrow |
| `inbox_capacity` | `10000`, exact durable Inbox entries; no carried-inventory preflight |
| `item_classes` | `["nonstackable","stackable","empty_container"]`, further narrowed by current admitted Content |
| `content_artifact_file` | `null`; configure the actual server-authoritative reference artifact |
| `content_artifact_sha256` | `null`; exact SHA-256 of that file, independently checked against current world activations |
| `gold_fee_bps` | `250` |
| `reserve_incoming_headroom` | `true` |
| `common_restore_scope` | `true`; all Bazaar, item, bank and Character state uses the same Game database/recovery fence |
| `snapshot_source_authority` | `"oteryn:game:primary"`; match `platform.bazaar.source_authority` |
| `signing_key_file` | `null`; configure an external regular private 32-byte Ed25519 seed file |
| `signing_kid` | `null`; explicitly provision an immutable key identifier |
| `signing_trust_generation` | `1`; explicitly re-provision current keys/revocations with the new admitted generation after recovery |
| `signing_public_keys` | `{}`; at most two explicitly provisioned current/retiring kid → lowercase 32-byte public-key hex bindings |
| `revoked_signing_kids` | `[]`; revocation is persisted monotonically and cannot be undone by a later configuration |
| `proof_retention_seconds` | `7776000` (P90D audit policy; unsettled authority/idempotency proof is never deleted by audit expiry) |
| `v2_receipt_bytes` | `270336` |

The signer loads the seed from `signing_key_file`, derives its Ed25519 public key,
and requires that exact key under `signing_kid` in `signing_public_keys`. Key IDs
are explicitly configured; they are not inferred from a payload or reused after
revocation. Platform loads the same public-only allowlist. No deployment public
key or AccountId is invented. Missing provisioning leaves the corresponding
operation PENDING while the enabled consumer continues other work.

Synthetic E2E signing identity: `fixture-example-1`, public key
`e8c30390441a54642382f15a784087a42dc75f85f775fdfc9e2a25b0f08a9806`.
Use it only for the synthetic fixture verifier, never as deployment trust.
Snapshot signing input is `oteryn-v2-bazaar-character-snapshot-v1.` followed by
unpadded base64url of the exact snapshot bytes.

Platform E2E stub fixtures:
`docs/contracts/fixtures/bazaar/README.md` and
`docs/contracts/fixtures/bazaar/decisions.json`. Exact strings exclude fixture JSON
framing and the file's final newline. Existing CC v1 settlement bytes remain on
the existing Character command feed/receipt route.

ANL feed/event family is the existing `BAZAAR_AUTHORITY_DECISION` (event type 6,
schema 1), in the existing `EventEnvelope` with `TransactionEventRef`,
`DURABLE_AUDIT`, `RESTRICTED_PLAYER_LINKED`, and
`PLAT3_AUTHORITY_AUDIT_RETENTION_V1`. There is no newly allocated Gateway HTTP
path or topic. The Game retains complete bounded envelope bytes atomically with
the decision through the same durable read/ack discipline as existing audit
owners. The six Bazaar POST routes above carry custody/policy/completeness;
terminal receipts use the separately authenticated Character receipt route.

### Compatibility decisions

- The source tree had lifecycle `1` only despite the candidate referring to an
  existing locked lifecycle. Migration 0994 adds internal lifecycle `2` for the
  contract's `locked` state, with exact same-transaction transition proof and
  frozen Character writer guards. No registered wire ID changes.
- Accepted CC v1 transferable lifecycle semantics remain available. Crossing a
  Bazaar-locked lifecycle requires the exact stored binding; no generic locked
  bypass is introduced. Bazaar listing/settlement uses that lock-bound path.
- Bazaar snapshot/status/policy/watermark epochs use the reconciled nonrollback
  recovery generation. CC v1 receipt projection epochs retain their existing
  LCFA scope and field semantics.
- The native mint owners store the known typed item-family tag `Item`; the
  candidate wire spells that enum tag `item`. The producer serializes only these
  two known representations as wire `item`. It preserves the complete original
  database definition in custody/split proofs and never rewrites the instance.
  Unknown families and out-of-grammar keys/revisions are refused. Current known
  Content transferability and exact full attribute conservation remain required.
- The supported Character capture profile refuses equipment, allocated Wheel,
  active condition or unresolved obligation state it cannot faithfully export.
  Optional unsupported tabs remain explicit null. Derived required fields come
  from the existing gameplay owners, not configuration-supplied statistics.
- The candidate's default-off instructions and lock rollout wording are
  superseded for this Game producer by the owner's 2026-11-06 default-on ruling.
  Contract candidate status and Platform qualification are not fabricated.

### Recovery evidence

Exact retained receipts replay across recovery generations without re-signing or
rewriting their bytes. New disposition and payout still require current-generation
evidence, as contract section 3 requires; an old receipt alone cannot authorize a
new debit, transfer or release. Common database restore scope is mandatory.

Signing trust is explicitly bound to `signing_trust_generation`. Restoring a backup
cannot re-enable its old signing policy under a newer nonrollback recovery fence:
fresh capture and snapshot trust remain PENDING until Operations re-provisions the
current external key/revocation policy for that generation. Database tombstones are
monotone within a generation; they are not claimed to survive arbitrary database
rollback independently of that external provisioning. Destroy retired seed files
and retain current revocation policy outside the restored Game database.

Restore admission independently checks current escrow custody and frozen attributes,
retained lock/ownership transitions, hold disposition and bank headroom, and exact
linked payout ledger conservation against Game-owned receipts. Historical terminal
items may subsequently move, transform or change attributes through their native
owners; recovery validation uses their immutable transition history rather than
requiring them to remain in the original Inbox. Active-only unique indexes prevent
double custody while preserving prior terminal listings when an asset is resold.

Local Content acquisition is bounded to one blocking reader per process and a
10-second total queue/read deadline. The blocking worker retains its permit after
timeout, so an unavailable filesystem cannot accumulate readers. Timeout leaves
PENDING without any SQL transaction held.

Gold Character payout uses the exact retained ownership-transition proof and its
unchanged settlement receipt, rather than the listing's mutable last-operation
field. Unlock may precede payout; it does not release the delivery-bound hold or
make a valid payout depend on the order of those two saga steps. Listing escrow
receipts never qualify as settlement. Current generation, sale, buyer, seller,
world, Character, reservation, revision, gross and receipt-digest checks remain
mandatory.

Snapshot public keys are also checked against retained admission signing-trust
keys, including retired admission keys. Provision distinct keys for these purposes.

### Publication and restore qualification

Retained terminal receipts are historical decisions, not fresh sale authorization.
Lock receipt publication extracts the exact embedded signed bytes with `RawValue`
and verifies their provenance using the immutable recorded kid/public-key binding,
including a subsequently revoked key. It does not re-sign, reorder or require a
new current signing policy to replay a historical receipt. Current eligibility,
Bind and settlement still require current trust and recovery evidence.

After recovery, retained terminal custody statuses publish under the current
projection floor with their historical snapshot hash unchanged. An active lock
whose capture belongs to an older floor publishes `INVALID` with
`RESTORE_RECONCILING`; other lost policy/trust qualification also publishes an
explicit invalid reason. Neither a terminal status nor that historical hash makes
an old capture eligible for a fresh sale. This permits complete bounded resync
without suppressing terminal history forever.

Publication acquires the same Content and complete admission serialization gates
before projection state, outboxes, custody rows or signing-trust reads. This fixes
the cross-node source/outbox lock inversion while retaining authoritative writer
fences. Acknowledgements still compare exact epoch/revision/body digests.

A verified terminal refusal clears the mutable disposition fields when releasing
all or part of a pending GOLD reservation. Its remaining absolute maximum can
then be replaced normally; immutable refusal/binding/release history is retained.
Internal `PAID` remains distinct to prevent a second disposition; the normative
GoldReleaseFacts wire state remains `ACTIVE` for every nonzero remainder and
`RELEASED` exactly when zero.

Restore qualification checks both directions: surviving holds must have their own
receipts, and retained GOLD handoffs/bindings must have the exact compatible hold,
delivery or payment/refusal history. A surviving delivery cannot reopen over a
missing hold or an earlier unfenced `ACTIVE` hold. Qualification tests simulate
those invalid partial restores only in isolated test databases using ordinary
administrative SQL, restore the exact original row, and requalify before any
further producer action. No trigger is disabled and no accepted fixture funds are
created by those corruption probes.

The durable snapshot and actual node adapter share the same 24-record batch
constant. The dispatch qualification invokes that production snapshot boundary,
reads its exact persisted receipt and verifies a durable digest-matched ACK; a
negative control with the former 32-record caller fails before command polling.

## Delivery gdel

# GPROD-DELETION — Game producer

Base: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`. Game implementation only;
Platform adapters and cross-repository E2E qualification remain Platform work.

## CONTRACT-NOTES

The owner's 2026-11-06 decision supersedes the candidate's default-OFF rollout
wording: Game's producer is enabled by default, with an explicit disable switch.
This does not assert deployed Platform qualification or authorize deployment.

The candidate requires independently current lifecycle evidence but supplied no
wire for it. Game adds the authenticated lifecycle read below. Its response is
independent of immutable intent fields; expected lifecycle/security/recovery values
are never treated as current evidence. A failed read invalidates cached evidence.
Platform must mirror this small extension. Security remains the existing S1/S2
source; known credential revocation during termination is not a financial blocker.
The lifecycle response separately attests `financial_obligations_clear` (required
boolean; false holds CLOSE/FINALIZE pending; CANCEL can compensate).

Immutable creation roots remain private quarantine backing. The closure membership
relation detaches active ownership without falsifying creation receipts. All active
Character reads, LCFA, profile, online and highscore projections exclude detached
members; root and custody writes are fenced. CANCEL restores the original relation,
revisions and custody hash, without reconstructing items. A runtime owner mints an
unforgeable absence capability only after scanning all actor slots; durability also
requires terminal session, released claims and current scope/node ownership. A kick,
disconnect, timeout or expired lease alone never proves absence.

Unknown reversible domain operations stay PENDING. Defaults accept only independently
vacant house/guild/market/bank domains; occupied house/guild rights, market escrow or
nonzero bank balances or outstanding guild disband claims cannot produce a successful close. No destructive substitute,
automatic succession or bank redistribution is selected. `refuse` for house/guild
turns a confirmed held right into registered OBLIGATION. CANCEL keeps all admission
closed until the entire original manifest is restored. Nonempty FINALIZE requires
configured physical-retention acceptance, name retention and public handoff completion;
empty FINALIZE can complete and records a pending erasure job. Erasure jobs do not
claim the owning domain has physically erased records.

Authoritative receipts and minimal fences are retained independently of P90D family-7
audit. `proof_retention_seconds` is reserved policy metadata: setting it cannot expire
replay evidence. Physical erasure remains `PENDING_DOMAIN_RETENTION`; no deletion of
financial evidence is inferred from account closure. Restoration conflicts remain
closed/PENDING; `restoration_operator` identifies an owner, never grants a bypass.

Migration: **0996** (checked free against main/open PRs). Existing event family **7**,
schema **1**, and errors **5014–5016** are reused. No new capability bits, numeric
command IDs or protobuf message numbers. Resource fragments **1698–1713** register
bounded producer envelopes; no pins or goldens were edited by hand.

## PLATFORM CONFIG

There are **no new runtime environment variables**. Game reads TOML via the existing
`serve --config <file>` interface. All keys below have prefix
`platform.character_commands.account_deletion.`:

| Key | Shipped default |
|---|---|
| `enabled` | `true` (false refuses new effects/PENDING; terminal replay remains available) |
| `epoch_fence_file` | `/var/lib/oteryn/account-closure/epoch-fence` |
| `lifecycle_source` | `platform_pull` (`pending` disables positive evidence; `control_observation` uses independently supplied control observations) |
| `house` | `vacant_only` |
| `guild` | `vacant_only` |
| `market` | `vacant_only` |
| `bank` | `vacant_only` |
| `preserve_custody` | `true` |
| `permit_retirement` | `true` |
| `name_release_after_seconds` | `2592000` (30 days) |
| `physical_erasure_after_seconds` | unset (`None`, domain acceptance pending) |
| `proof_retention_seconds` | unset (`None`, no authoritative replay expiry) |
| `restoration_operator` | unset (`None`, conflict remains closed) |

TOML omits optional unset keys (no `null`). The supplied `producer.example.toml`
installs the command identity section and explicitly enables deletion. The enclosing
Character command identity must be provisioned; a missing identity cannot be replaced
by the security-evidence identity. Existing required keys have no fabricated defaults:
`platform.endpoint` (IP:port), `platform.peer_name` (TLS DNS name),
`platform.trust_roots_file`, `platform.character_commands.source_authority` (configured
Game Character authority namespace), `platform.character_commands.client_certificate_file`
and `platform.character_commands.client_key_file`. Other required server settings remain
those of the existing node config. The optional old CC identity section is unchanged;
when installed, deletion is ON even when its subsection is omitted.

**Public key IDs:** no JSON/JWS `kid` or application public-key ID exists in this
contract. TLS leaf identity is loaded from the configured PEM chain; the PEM private
key must match that leaf. Platform authorizes its pinned/provisioned certificate/SPKI
under purpose `OTERYN_GAME_CHARACTER_COMMANDS`; the descriptor rejects certificate
reuse across other installed purposes. Platform server identity is validated with the
configured trust roots and `peer_name`. Synthetic fixture UUIDs/hashes are not keys.
No key is generated, inferred from a UUID, or embedded in fixtures. Platform must use
the same provisioned Character identity mapping as CC, rather than invent a new kid.

**Transport:** HTTPS/mTLS POST to `platform.endpoint`, Host/TLS name
`platform.peer_name`, purpose-separated Character command identity. All requests use
canonical ordered UTF-8 JSON. Existing CC pending/read/receipt routes carry Close and
Resolve. The command pending-list/read wrappers remain v1; immutable deletion intents,
receipts, private closure snapshots and watermarks are v2.

| Operation | Method/path | Maximum body |
|---|---|---:|
| List pending | POST `/internal/v1/game-auth/character-commands/pending` | existing CC limit |
| Read immutable intent | POST `/internal/v1/game-auth/character-commands/read` | intent response 4096 |
| Publish terminal receipt | POST `/internal/v1/game-auth/character-commands/receipt` | 2048 |
| Read current lifecycle | POST `/internal/v1/game-auth/account-closure/lifecycle/read` | request 512; response 2048 |
| Publish private snapshot | POST `/internal/v1/game-auth/account-closure` | 2048 |
| Publish watermark | POST `/internal/v1/game-auth/account-closure/watermark` | 512 |

Feed/family name: **`account_closure`**. No Kafka/AMQP topic or additional broker auth.
ANL durable outbox family7 uses existing bounded collector read/ack discipline,
`PLAT3_AUTHORITY_AUDIT_RETENTION_V1`, P90D and case-scoped legal holds. It is separate
from the Platform private feed and authoritative receipt retention.

Lifecycle request operation is `ReadAccountClosureLifecycleV1`, source is configured
Game namespace, audience `OTERYN_PLATFORM_CHARACTER_AUTHORITY`. Response source is exactly
`OTERYN_PLATFORM_CHARACTER_AUTHORITY`; canonical
field order and examples are in `lifecycle.json`. Required phase is CLOSE/CANCEL/FINALIZE,
positive decimal lifecycle/source/recovery counters, UUIDv7 account/termination IDs,
`observed_at <= Game now < valid_until`, and validity at most **5 seconds**. Platform
uses its independently locked current lifecycle, echoes the requested current Game
recovery only after reconciliation, and advances observation `source_revision` for a
new observation/time. This revision namespace is independent of immutable intent
`source_revision`. Exact duplicates are usable only within their original validity;
unknown, mismatched, stale or malformed evidence remains PENDING. No source deadline
or retention counter is extended by retry.

Projection ack: exact `{"contract_version":2,"result":"accepted"}` or `superseded`.
Receipt ack requires `accepted`; receipt404 or conflict409 quarantines that publication until explicit
resync. Epoch high-water file is external to Character backup: provision `0\n` only on
a genuinely fresh deployment, then retain it across recovery. Missing/malformed files
block publication. Control `game_account_closure_resync(new_epoch)` must select an epoch
above Platform's accepted floor after current recovery reconciliation; snapshots and
watermarks refuse a recovery generation until this explicit control resync. Historical
receipt requery remains byte-identical. Bounded full enumeration republishes original linked
receipts before new outer-epoch snapshots. Proof bytes are never rewritten.

**Platform E2E stub fixtures** (relative to repository root):

- `docs/contracts/fixtures/account-deletion/closure.json`: all original exact wire strings
  and SHA-256 values, including Close proof, private projection and cancellation-before-close.
- `docs/contracts/fixtures/account-deletion/lifecycle.json`: exact request/response vectors
  and hashes for independent lifecycle reads. Times are synthetic; use a controlled clock.
- `docs/contracts/fixtures/account-deletion/producer.example.toml`: ON config fragment;
  PEM paths are provisioning placeholders, not credentials.

## Evidence and remaining owner choices

Focused PostgreSQL cases exercise durable restart, fixture-byte output, replay,
exclusive resolution, stale node/recovery, real actor retirement/released claims,
partial detach then restart/CANCEL, LCFA lag, epoch resync, FINALIZE, audit expiry and
legal holds. Exact filters and measured durations are recorded in the delivery report.
No full PostgreSQL suite or full nextest is required.

Open choices remain configurable: occupied house/guild reversible adapters; market/bank
final disposition; physical retention acceptance; authoritative proof-retention policy;
restoration operator; request-time Platform blocker relaxation (unchanged); PKI/deployment
provisioning and cross-repository exact-head qualification. They are not bypasses.

Read-only donor fetches succeeded for Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`
and CrystalServer `201189f871d23597b82caf88fd611454a2ddefb8`, both
`src/io/iologindata.cpp`. Their legacy persistence does not define Platform authority.
Wiki checks: **wiki-pending**. NEED-FETCH: none.

Read-only comparison against `claude/happy-davinci-ijz3wy-srvpkg4` at
`5e60c19701fe3acdda0a3d1331da1239440a9d1e` found no deletion producer to duplicate.
Shared host/authority/carrier files receive only additive deletion hooks; no NX, SRX,
WORLD-SRC, quest, casting or other gameplay-row implementation is imported.

The Game-side review clarified three edge cases without weakening fences: an early
FINALIZE stays PENDING until its matching Close commits; reissuing a completed
CANCEL after recovery reconfirms its historical outcome/current LCFA pair without
undoing subsequent gameplay; and pre-existing Market owners may finish their
conserved positive-credit settlement before detach. The bank exception requires
current-transaction Market settlement/entry evidence and a pre-existing owned offer;
new offers and unrelated bank operations remain refused. These are Game producer
corrections, not new Platform decision authority.

Private publication polls independently once per second, with one in-flight mTLS
exchange in that publisher and bounded 429/503 backoff up to ten seconds. Normal
watermark attempts become due after five seconds (within the contract's ten-second
cadence); blocked changes never advance completeness. The separate CC consumer
retains its own one-exchange bound. A slow/unavailable pending list cannot starve
private receipt replay or heartbeat attempts.

LCFA, profile and affected-world highscore publication hold restored members while
CANCEL is partial. This prevents publishing a revision whose contents would change
without a revision increment when the final restoration transaction reopens admission.
Unreconciled nonterminal leases remain PENDING; no new timeout/absence inference
bypasses their existing admission/session release owner.

The 256-job pending envelope reserves 128 slots per command family. A full Close
backlog cannot consume CANCEL/FINALIZE capacity. Receipt/snapshot acknowledgements
and conflict quarantine bind both the indexed identity and exact stored body;
paged delivery/expiry/source/work queries have matching partial indexes.

The original `scheduled_for` is durably bound per account/termination/Close generation,
separately from technical attempt TTL. A fresh operation key cannot move that deadline;
a contradictory retry receives `CHAR_CMD_INTENT_INVALID`. A delayed watermark conflict
from an older epoch cannot quarantine an explicitly resynchronized newer epoch.

The shipped fragment also explicitly enables/provisions the existing dependent
feeds; their acknowledgements are required before closure freshness can advance.
For each prefix `platform.account_characters.` and `platform.public_projections.`,
keys `client_certificate_file`, `client_key_file`, `source_authority` and
`epoch_fence_file` are required (no fabricated credential defaults); exact delivery
values are in `producer.example.toml`. The public prefix also sets `enabled=true`.
Its older standalone contract's omitted-field default is false; this delivery's
shipped fragment overrides it ON. LCFA runs when its identity section is installed.
These credentials are distinct from CC and native evidence. Their mTLS purposes are
`OTERYN_GAME_ACCOUNT_CHARACTERS_PROJECTION` and `OTERYN_GAME_PUBLIC_PROJECTIONS`,
respectively; neither has an application `kid`. Their high-water files are separately
retained outside Character backup, with the same fresh-only provisioning rule.

Dependent transport, all HTTPS POST to the same configured Platform endpoint:
`/internal/v1/game-auth/native-account-characters` and
`/internal/v1/game-auth/native-account-characters/watermark`;
`/internal/v1/game-public/character-profile`,
`/internal/v1/game-public/highscores`, `/internal/v1/game-public/world-online`,
`/internal/v1/game-public/world-totals`, and
`/internal/v1/game-public/watermark`. They retain their existing v1 contracts and family names
`character_profile`, `highscores`, `world_online`, `world_totals`, and LCFA's account-character feed.
They do not use the CC identity. Closure does not bypass a disabled/unavailable
or unacknowledged dependent feed; it remains PENDING/stale.

## Delivery t94

# GPROD-T94 — Game delivery notes

Base: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`. Game only; no Platform files, commits or pushes.

## CONTRACT-NOTES

The owner's 2026-11-06 instruction supersedes the candidate's default-OFF rollout text: the shipped node template, deserialization default, producer and database control start enabled. Explicit disabling refuses publication. The contract lock remains candidate; this change does not assert that Platform has qualified its adapter.

The fixture sequence omits LOCKED between SCHEDULED and SETTLED. All supplied wires remain byte-exact standalone delivery goldens. Production inserts LOCKED and advances the revision before settlement; the disconnected schedule→settlement fixture pair is refused as an authority transition. No fence is relaxed to accommodate the fixture numbering.

There are no concrete owner-approved positive scoring rules or advance aggregation values in the open questions. Configuration retains these choices. Defaults publish actual reset/raid/boss history as ineligible VOID, never retrospective predictions. All target Channels are required for an approved advance binding; no first-Channel winner. `EventBinding.channel_policy` defaults to `UNANIMOUS_OR_PENDING`; an explicitly approved `ALL_TARGETS_OR_VOID` policy can withdraw a mixed terminal aggregate as SOURCE_UNAVAILABLE. Native cancellation and source loss intrinsically yield their VOID reasons, independently of positive option mappings. Boss settlement requires the actual placed boss-death receipt. Placement failure, timeout and lost ownership cannot stand in for a boss kill. Native despawning completion without the promised boss-death receipt produces native SOURCE_UNAVAILABLE evidence for a VOID fallback; a retained actual death receipt is preserved. No generic completion authorizes a killed outcome. Persistent bosses keep their source until actual death or source loss; closing a non-despawning raid schedule is not boss loss. An advertised event can lock and await a native receipt indefinitely; scheduled_end alone never settles it.

Default seasons use a persisted rolling UTC-second calendar: first start is bootstrap time +3 seconds, duration 86400 seconds (configurable range 60..31536000), freeze at end +1 second. Missed calendar periods are backfilled one per bounded pass; completeness is withheld until the calendar is caught up. Expired native tombstones suppress recapture rather than reopening old reset questions. The default challenge is `world:gathering`, ruleset/scoring revision `t94:zero-score-season-v1`, max_score=1, no positive source rules and no eligible guilds. This produces a real OPEN/FROZEN empty standings record, not fabricated guild points. Positive `SeasonBinding.rules` and eligibility must be approved before starts_at. A committed native kill uses a fixed rule award, deduplicated per season/challenge/occurrence/guild; paid/XP modifiers are not score inputs. Attribution defaults to `PRINCIPAL_ONLY` (the fenced native kill principal); an explicit pre-season `SeasonBinding.attribution=QUALIFIED_PARTY_ROSTER` can select the qualified native party roster. This does not use premium or final XP amounts as a participation test. Membership is captured inside the fenced source transaction and never reattributed by later joins/leaves. Source acceptance time is the database cut, rather than a delayed wall-clock combat timestamp.

Bindings are Game-owned database configuration, not Platform input. `SeasonBinding.attribution` defaults to `PRINCIPAL_ONLY`; allowed explicit alternative is `QUALIFIED_PARTY_ROSTER`. `game_t94_bindings.binding` stores the strict `EventBinding` / `SeasonBinding` JSON types; `approved` defaults false. Approval cuts are stamped by Game database triggers at insertion/false-to-true approval and cannot be backdated after approval. Correction acceptance after a declared closed deadline receives an immutable refusal; accepted earlier requests retain their approval cut on replay. The runtime creates conservative default bindings. Administrative positive bindings must supply exact native content, occurrence, source, immutable rules/calendar, options and complete target Channels. Published bindings and started seasons are sealed against rule/calendar/eligibility rewrites. `game_t94_corrections` accepts explicitly approved withdrawal/restoration of an existing contribution, never arbitrary scores; its terminal receipt is immutable. Final freeze refuses further corrections.

Unresolved decisions remain configurable: positive event mappings/windows/aggregation, eligible guilds, positive challenge rules/maxima, season calendar, correction deadlines, disband tombstone and actual Fury realization availability. Missing names withhold display (`[]`); they never zero scores or change authority to VOID. FROZEN records are immutable; incompatible disband display is withheld pending a versioned policy. A missing Fury realization produces SOURCE_UNAVAILABLE bound to an actual reset receipt; setting availability true does not invent a realization, and the world-family watermark remains PENDING for missing declared gate receipts. Source metadata cannot authorize an outcome.

Native receipts, immutable revisions, evidence, terminal receipts and publication/audit outbox commit under the existing semantic transaction and sealed recovery authority. A source capture semantic error rolls back its savepoint and quarantines T94 without fabricating a result. Projection ACK and audit ACK are independent. Completeness also waits for unprocessed disband display withdrawals. The Bazaar producer is absent from this base; integration must add its installed leaf to the mutual purpose-separation checks when its configuration is introduced, preserving the contract's dedicated-key rule. Event IDs 10/11 and payload tags from PR #3010 are unchanged. No capability bits, gameplay commands or public protocol message numbers are allocated. Migration: 0998 only. Resource fragment ordinals start 1698 and are integration-renumberable.

Delivery uses a durable ten-second family lease with a six-second remaining-time check before a five-second HTTP exchange. Restart retries the same retained bytes. A later LOST cleanup ACK replays the immutable native loss commit timestamp, rather than changing source evidence with ACK time. Native event decision time must be present and positive; observation time is never substituted for a missing native decision clock. A 409 quarantines the authority. Missing/unreadable epoch fence refuses delivery; it is never initialized by the producer. First-time empty-store bootstrap requires an explicitly provisioned trusted F=0 and sealed current Character authority; F>=1 refuses empty-cache bootstrap. Positive guild seasons cannot begin before native capture starts. The T94 file wrapper locks the stable parent directory and rechecks before every persist, refusing stale writers that would lower F. Explicit resync first quiesces both durable family delivery lanes (a busy lane yields a retried PENDING/refusal without changing epoch), requires a UUIDv7 request and advances beyond both database epoch and nonrollback F, persists F before sending, enumerates the complete retained correction lineage, and withholds completeness until staging is acknowledged. Ordinary restart does not mint an epoch. A restored nonempty store must first be reconciled by the Game recovery operator: `game_t94_control.recovery_generation` must equal the sealed Character generation and `reconciled=true`; the resync operation refuses to self-certify that restoration. Default policy configuration is hashed when first installed for a World; changing it requires an explicitly versioned binding/calendar rather than rewriting an active season. P90D expiry clears audit/source/display payloads in bounded batches while keeping minimal identity, hash, nonreuse and terminal controls.

## PLATFORM CONFIG

There are **no T94 environment-variable overrides**. The server uses TOML keys below. PostgreSQL tests use the existing `OTERYN_TEST_POSTGRES_ADMIN_URL` harness variable; it is not a production transport setting.

| Config key | Default |
|---|---|
| `platform.world_event_outcomes.enabled` | `true` (absent section also enabled) |
| `platform.world_event_outcomes.source_authority` | `oteryn:game:primary` |
| `platform.world_event_outcomes.client_certificate_file` | `/etc/oteryn/credentials/t94-client.crt` |
| `platform.world_event_outcomes.client_key_file` | `/etc/oteryn/credentials/t94-client.key` |
| `platform.world_event_outcomes.epoch_fence_file` | `/var/lib/oteryn-fences/t94.epoch` |
| `platform.world_event_outcomes.resync_request_id` | absent; explicit UUIDv7 requests resync |
| `platform.world_event_outcomes.defaults.fury_gate_realization_available` | `false` |
| `platform.world_event_outcomes.defaults.season_seconds` | `86400` |
| `platform.world_event_outcomes.defaults.season_advance_seconds` | `3` |
| `platform.world_event_outcomes.defaults.freeze_delay_seconds` | `1` |
| `platform.world_event_outcomes.defaults.event_window_seconds` | `1` |
| `platform.world_event_outcomes.defaults.correction_seconds` | `1` |
| `platform.world_event_outcomes.defaults.max_score` | `1` |
| `platform.world_event_outcomes.defaults.eligible_guilds` | `[]` |
| `platform.world_event_outcomes.defaults.tombstone_name` | absent (withhold unknown disband display) |
| `platform.endpoint` | required deployment SocketAddr, shared existing Platform endpoint; no invented host/port |
| `platform.peer_name` | required existing verified TLS server name |
| `platform.trust_roots_file` | required existing PEM trust roots path |

The Synology template explicitly enables T94 and uses `<BASE>/node/secrets/t94-client.crt`, `<BASE>/node/secrets/t94-client.key`, and `<BASE>/t94/epoch-fence`. These are deployment substitutions for the loader defaults above. Credentials and the initial nonrollback fence must be provisioned by the deployment owner; no embedded credentials, auto-issued trust or fallback identity exist. Keep the fence outside Character database backups. All publishers of the same source namespace must share the same nonrollback fence and a stable lock-capable parent directory; a node-local copy is not an HA substitute. An empty T94 store initializes under the sealed current Character generation only with trusted F=0. New T94 stores use projection epoch 1; provision the initial trusted F=0 file (explicit `0` or `0\n`, meaning no publication has occurred) under the existing EpochFenceFile ownership/durability rules.

Public key ID is exactly `sha256:` followed by lowercase hexadecimal SHA-256 of the **leaf certificate DER SubjectPublicKeyInfo**, exposed as `LeagueDescriptor::public_key_id()`. It is derived at credential load, not an arbitrary configured kid, and changes on key rotation. The matching private key loads from the dedicated key file. Its SPKI must differ from evidence, runtime status, account characters, character commands and public projections. Purpose name is `OTERYN_GAME_WORLD_EVENT_GUILD_SEASON`. Platform must authorize that dedicated mTLS client identity for source namespace `oteryn:game:primary` and these operations. There is no JWT/bearer signing kid, Authorization header, browser route or inbound Game listener.

| Method | Endpoint path | Feed/family | Maximum body |
|---|---|---|---|
| POST | `/internal/v1/game-public/world-event-outcome` | `world_event_outcome` | 16384 bytes |
| POST | `/internal/v1/game-public/guild-season` | `guild_season` | 262144 bytes |
| POST | `/internal/v1/game-public/league-watermark` | watermark for either family | 512 bytes |

Transport is Game-initiated HTTP/1.1 over TLS1.3 with dedicated mTLS and verified Platform peer/trust roots, JSON canonical bodies, one in flight per family. These are HTTP feeds, not broker topics. Responses require `Cache-Control: no-store`, at most 256 body bytes; non-200 bodies are empty. Statuses are 200/400/401/409/429/503. The existing standard PLATFORM error codes 5002–5007 are reused. Retry 429/503 with retained identical bytes and bounded 1/2/4/8/10-second backoff. Connect 1 second, TLS handshake 2 seconds, complete exchange 5 seconds. No per-subject evidence or payload is logged.

Platform E2E stub inputs (unchanged):

- `docs/contracts/fixtures/world-event-guild-season/snapshots.json`: every canonical wire and SHA-256, including schedule, settlement, VOID, correction, final freeze and tied frozen guilds.
- `docs/contracts/fixtures/world-event-guild-season/README.md`: normative replay/recovery/retention scenarios and conformance requirements.
- `docs/contracts/game-events/v1/plat3_authority.proto`: audit payload definitions; family IDs remain in `GAME_EVENT_FOUNDATION_REGISTRY.json`.

Fixture publication tests replay every supplied wire through the publisher and retry/ACK path byte-exactly. They do not turn fixture hashes into native evidence or treat the omitted LOCKED revision as an allowed production transition.

### Game-owned binding configuration

These JSON keys are local Game configuration in `game_t94_bindings.binding`; they are not additional public wire members. Platform mirrors the resulting ruleset meaning and published snapshots.

| Key | Default / requirement |
|---|---|
| `EventBinding.channel_policy` | `UNANIMOUS_OR_PENDING`; explicit alternative `ALL_TARGETS_OR_VOID` |
| `EventBinding.advance_approved` | `false` |
| `EventBinding.channels` | `[]`; a raid/boss advance mapping must name every actual target Channel |
| `EventBinding.outcomes` | `{}`; positive native phase to approved option mapping |
| `EventBinding.void_reason` | absent; explicit no-advance history uses `INVALID_SOURCE`; native cancellation/loss intrinsically carry their VOID reasons |
| `EventBinding.corrections_closed` | `false` |
| `EventBinding.correction_deadline` | absent; no final freeze without a finite configured deadline and complete cut |
| `EventBinding.withdraw_display` | `false` |
| `EventBinding.occurrence_id`, `question_key`, `question` | required actual occurrence, permanent question identity, immutable strict event template |
| `SeasonBinding.attribution` | `PRINCIPAL_ONLY`; explicit alternative `QUALIFIED_PARTY_ROSTER` |
| `SeasonBinding.rules` | `{}`; fixed unpaid award per qualified native creature key |
| `SeasonBinding.eligible_guilds` | `[]` |
| `SeasonBinding.tombstone_name` | absent; withhold unknown disband display |
| `SeasonBinding.corrections_closed` | `false` |
| `SeasonBinding.correction_deadline` | absent; no final freeze without closure |
| `SeasonBinding.complete_through` | `0` (PENDING until a qualified complete cut exists) |
| `SeasonBinding.withdraw_display` | `false` |
| `SeasonBinding.season` | required immutable calendar/challenges template; initial standings empty |
| `game_t94_bindings.approved` | `false`; approval time is stamped by Game and sealed |
| `game_t94_corrections.approved` | `false`; approval cut is stamped, terminal receipt immutable |

`platform.world_event_outcomes.enabled=false` stops publication and interpretation. Already-qualified native capture can retain source inputs for later replay; retention runs independently. Source configuration that lacks accepted evidence stays PENDING. Positive seasons that start before capture bootstrap are refused rather than assigned invented zero scores.

## External evidence

Read-only donor evidence fetched successfully:

- https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/world_changes/fury_gates.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/fury_gates.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/raids.cpp

Fury Gates selects/realizes at startup; donor content is not an accepted Game realization receipt or an advance prediction window. Wiki checks: **wiki-pending**. NEED-FETCH: none.

## WPKG — WORLDCHG-Q merged and focused validation complete (2026-11-07)

Merged source `ebc53bc6e` as merge commit `718b199fb`; follow-up `d9dacb214` repairs native-model generation order. Main's Fury/Nightmare layers and Flower/Stampede raid test branches are retained. The captured-wolf profile reuses main's canonical owner: 40 new native profiles, no removed or duplicate keys, and the existing wolf record is byte-semantically unchanged. All eleven lane-owned titles remain QUALIFIED (the catalogue total of twelve includes the existing Stampede).

Source conflicts were resolved in daily-state persistence/layer projection, boot loading and raid announcement fixtures. Derived content, manifests, callbacks, native providers, Grizzly room input pins, Prey model/reports and world pins were regenerated with repository tools. The Prey producer now writes generated loot admission before hashing its model input; its regression requires the generated closure to be visible. No generated file was hand-edited.

Focused validation (command wall durations; logs retained under `/tmp/wpkg-*` in this workspace):

- `worldchg-fmt`: PASS, 17.036s.
- `worldchg-check`: PASS, 27.052s.
- `worldchg-clippy`: PASS, 259.540s.
- `worldchg-quest-final`: PASS, 384.840s.
- `worldchg-native-check-final`: PASS, 83.119s.
- `worldchg-prey-tests`: PASS, 0.170s.
- `worldchg-pin-refresh-final`: PASS, 134.012s.
- `worldchg-pin-check-final`: PASS, 169.874s.
- `worldchg-content-integration`: PASS, 36.863s.
- `worldchg-content-world`: PASS, 86.877s.
- `worldchg-owner-pg`: PASS, 98.752s.
- `worldchg-dispatch-pg-focused`: PASS, 268.137s.

Content integration passed 123 tests (3.12s actual tests); world repository passed five (84.09s). All six owner PostgreSQL cases executed and passed (14.94s actual tests). The dispatch filter passed fourteen tests (193.16s actual tests), including thirteen native PostgreSQL cases with WORLDCHG-Q/RAIDS_GLOBAL execution markers. Case markers include Thawing51.776–71.725s, Voice54.880/68.648s, Grimvale45.743/57.326s, Hive66.131s, Benevola96.635s, Yasir102.670s and Yeo81.307s. The queued duplicate Cargo invocation was cancelled before any case executed; the actual run used six workers. No full PostgreSQL suite ran. The optional broader daily-map module was not run.

The fresh pin-refresh and pin-check bind digest `7d31c6eb0395cafda888178a0d5e204bad469be754430172a2f396c5e330daaa` (32,839,521 bytes). All 93 quest-authoring modules passed. Rust checks cover all targets/features of the eleven changed crates; Rust source remained unchanged after them. No PR is opened. WVAR follows next; its completed Fury Gates qualification must be composed with main's existing native layer/raid path.

## Coordinator migration registry — WPKG (2026-11-07)

WPKG owns 1330–1349. The five incoming sources contain 29 package-owned migration files, so related unmerged SQL is consolidated in original order; no accepted main migration is renumbered or edited. SQL statements are preserved. QENGINE2 PR3020 is still open at `efad3dee13e9cbc9652296726bf15828769c69d4`; its 0999–1006 content must remain canonical dependencies, never package variants. Shared QBASE files, if present, use the coordinator's 1200–1209 numbers and unchanged shared SQL.

Current WORLDCHG migration files:

- `apps/game-server/migrations/1330_world_change_owner.sql`
- `apps/game-server/migrations/1331_world_change_credit_counters.sql`
- `apps/game-server/migrations/1332_world_change_voice_support.sql`
- `apps/game-server/migrations/1333_world_change_barter_custody.sql`

1330 retains the World owner; 1331 retains credited counters. 1332 concatenates the gobbler cooldown and Voice phase migrations; 1333 concatenates Benevola custody and Yeo barter. Every statement remains in its original lane order. Exact filename references were updated. The repository duplicate-version check on the staged tree returned no duplicates. Focused PostgreSQL checks and regenerated pins follow this compiling checkpoint. Future lane migration groups will continue in ascending order inside this same package range.

Registry revalidation completed on the 1330–1333 allocation: all-target/all-feature
cargo check passed175.240s; all six focused owner PostgreSQL cases executed and
passed (12.37s test time,157.583s command). Tool-driven pin-refresh136.989s and
pin-check152.670s passed; digest remains7d31c6eb0395cafda888178a0d5e204bad469be754430172a2f396c5e330daaa.
The repository duplicate-migration check on HEAD returned no duplicate versions.

### WVAR compiling checkpoint: reset ownership and source materialization

The merged server/dispatch all-target/all-feature check passed (247.817s).
WORLD fixtures now explicitly commit only their current intended reset through
`record_reset_fixture`; future histories and boot/lazy reads remain inactive.
The main Furyosa regression chooses an actual Thais reset and retains stale,
exit-only, undated, firing and replay assertions. WVAR corpse import accepts
unadmitted legacy records without semantics while rejecting admitted conflicts
(five tests,0.526s). NAV keeps sealed pre-admission evidence (seven tests,24.406s).
Shared shop/potion input repinning permits only exact verified WVAR source
qualifier outputs from main9668; all other selected changes remain errors.
Generated content and focused PostgreSQL validation remain pending here.


## Integrated WVAR source contract notes

## WVAR spawn prerequisites

A gated spawn source opens after one sealed, committed death from each exact prerequisite home cell and creature identity. Repeated kills of one source, administrative removal, failed projection, and replay cannot replace another prerequisite. The existing spawn owner handles warning, occupancy, respawn, and activation rollback; no second scheduler is introduced. Prerequisites are local to the admitted Channel spawn set.

## WVAR variant projection and conservative source rules

Variant-only sectors use the same sorted tile stream and portal/marker binding as base sectors. A missing managed portal anchor refuses admission. Only bundles declaring the W1 capability receive these projections; exits remain available after closure through current authority checks.

Missing exact entrance geometry is represented by bounded, declared landing pads. Awash uses a separate dry return pad so a flooded entrance cannot strand a player; Down the Drain reserves dry entrance/return pads over the sourced surface water. Awash's202coal threshold is interpreted as the community total across three pumps. The exact source citations and per-title geometry assumptions are retained in the recipe's source/assumption fields.

## WVAR native creature and Item admission

Mamma Longlegs uses the retained1800HP/2200XP/outfit208 facts and the existing Giant Spider executable AI as the conservative archetype. Its new native profile invents no loot, Bestiary, Prey or Forge reward. The bounded corpse Item dependencies are admitted through the existing authoring importer. Sandfish13992 keeps its existing canonical weight660, stack100 and food regeneration660seconds; only the missing native provider binding is added. Source locators and hashes are recorded beside those native facts.

## WVAR camp, Spider Nest and illusion rules

Spider Nest entry arrives beside Mamma's live spawn cell and returns to a separate maintained surface pad. Decorative web12747 has no retained solid flag; the selected sealed variant therefore declares a bounded barrier through the existing map collision/passability owner. Three lifetime sealed credited Mamma kills deny new entry while the return remains available.

Bored uses three conservative adjacent illusion pads around Wyda because the retained source fixes the exact illusion identity/top-damage achievement but not an executable population count. The NPC home cell remains free. The existing seven-spawn Warpath recipe means six distinct guard homes before Bibby, using the already-pushed spawn prerequisite owner. Lumberjack's source50gold offer is interpreted as the player selling one piece of5901wood, independently of the Chip→Queen report.

## WVAR Lumberjack NPC contract

Retained TibiaWiki page76247/rev893906, `npc_conditions`:9–13 (body SHA256 `a31cf523648ff168e34f12a31429a05d8db42be4e7a0daa6df49dc6e32355ce2`) supplies the Chip→Queen Eloise order, Whistle-Blower, and a50gold wood sale with unspecified quantity. Wood5901 is Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `data/items/items.xml:15919`, file SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`.

The explicit conservative offer is that the player sells ONE wood5901 to Chip for50gold through the existing NPC trade owner. This is independent of the report and does not make50gold a quest reward. Conservative dialogue keywords are `wood`/`logging` at Chip and `chip`/`lumberjack` at Queen Eloise; replies are authored text, not purported donor quotations. Reporting requires the prior visit and grants Whistle-Blower once. It remains available after the daily change ends. Chip's authored home `[32418,31699,7]` is an explicitly inferred placement, checked against the current stored variant; wandering changes talk range, not source membership. New writes require a private context resolved from the real current NPC actor, session, lease, scope generation and active content. Existing matching Quest/trade receipts retain ordinary read-only replay after closure.

## Warpath repeat admission

The retained Warpath spoiler fixes24hours and restarts the clock on an early revisit. Entry therefore records an actual server movement-step Quest receipt: elapsed admission uses SetNow, while an early denied attempt uses a separate SetNow transition. A bounded palisade enclosure and separate return platform are conservative geometry for the retained Femor camp. The source inside roster remains two Berserkers, three Warlords and one Leader; only six distinct sealed guard deaths unlock Bibby through the existing spawn owner. Return never consumes or checks the entry clock.

## WVAR source-specific reward closure

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` fixes Wyda loot at `data-crystal/monster/raids/giant_spider_wyda.lua:67-70` (SHA256 `d64925aa3300e7bb6c8ec8abcfd815500624d344e4e2b411f8559624108993d6`): gold3031 and spider fangs8031 only. Bibby uses `data-crystal/monster/bosses/bibby_bloodbath.lua:72-90` (SHA256 `62bc99a21a214baaf6e1b0a8dba5ee595674c7baf15b0b3f952925ea3b58e5a2`), its17 genuine leaves; modern imported44612 is absent. These two source-specific tables replace the imported tables without promoting ordinary Giant Spider loot. Only six missing genuine leaves8031/7412/7395/817/3316/3265 become Native materializable Items.

The same pinned `data/items/items.xml` (SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`) supplies Bibby corpse6008 at16475–16480 and its4148→4149→4147 chain at12738–12755: capacities10 on the first three, durations10/300/300/60seconds, and weights0/120000/120000/10000. Pinned appearances preserve the primary corpse immobility and the later stages being takeable. Existing Item decay/custody owns the chain; no new scheduler. The existing independent Crystal/Canary imbuement cross-check gains only butcher axe7412, whose two slots and five family tiers match both exact source pins; no existing witness row changes.

The six leaf definitions in that same exact XML are8031:21688–21692,7412:19911–19930,7395:19688–19691,817:2728–2740,3316:9014–9024, and3265:8282–8295. The canonical Item source facts feed the native loot admission for Bibby corpse6008 (materializable, capacity10); later corpse stages retain the existing Item decay owner.

## WVAR generic physical reward closure

`tools/content-schema/native-gameplay/wvar-reward-closure.json` is the reusable source receipt for all42 WVAR recipe/source creature keys:41 retained donor Lua definitions plus the explicitly authored Mamma template. It derives133 corpse-chain Items and337 actual corpse/loot dependencies from the exact Crystal201189f archive SHA256 `2b734db10c8c00e7a11a59f1cb49a786099959e98a622732b7fb80bcc8345b9d`. Every creature has its Lua file:line+SHA; every Item has its XML file:line+SHA and span digest. No raw source or map bytes are copied into the repository. The bounded supplemental keys are the four source Shadowthorn Elf types and the prepared-bucket Thornfire Wolf.

The generic reward projection admits physical/stack/container/absolute decay facts and supported loot charges. Equipment, combat, imbuement and use-effect systems retain their existing owners; this reward importer does not fill their unknown fields. Existing canonical Known facts remain authoritative. Actual source primary-corpse/loot edges alone enter the loot section; decay-only stages and future bucket operands do not become unrelated loot facts.

Our earlier16-slot Mamma/Giant Spider projection was a native projection, not the donor capacity. It is corrected to the source30/30/20 capacities. The existing shared corpse-container owner now supports the exact required maximum44, rather than clamping larger bodies: Hive-related bodies and Deepling Scout36, Spectre/Poacher40, and Insectoid Worker44. Migration1102 retains the installed receipt/lifecycle/nested/single-location guards and extends their shared bound, including retirement ordinals. The independent total loot-plan ceiling remains unchanged.

Canonical guard receipts are regenerated through their existing source capture functions; their captured record sets remain unchanged. The existing Prey producer is rerun because its model binds the exact loot bytes. There is no second counter, store, admission table or timer in this source closure. The shared dependency closure does not qualify any additional world-change title by itself.

## WVAR iceberg, Spirit Grounds, Hive and Poacher rules

Chakoya's 63 retained donor homes use the 2,118 bounded snow/ice tiles recorded in the authored recipe. Equal iceberg/open-water alternatives are conservative: open water removes owned upper-floor terrain and changes floor 7 to appearance 622. The retained [Chakoya page](https://tibia.fandom.com/index.php?curid=76213&oldid=887139), line 4 (SHA256 `16abb4b72959e6969dc11e24717b7cbf703df6d7cc9ed4a8935686a4ff6a2e68`), declares no event reward. Its line 6 (SHA256 `ff4d57a94bdf28759e74d23b6806a02e949c1c2c78983d805a24fd0804d4671e`) distinguishes Cartography 101 and incidental Fireproof Horn loot.

Spirit Grounds retains all three entrance coordinates and the shared destination. Twelve equal variants are the Cartesian product of three entrances and four populations. This preserves the retained [Spirit Grounds page](https://tibia.fandom.com/index.php?curid=76264&oldid=887160), line 17 (SHA256 `59b829915a0b21c494521fb1a351335f44708cadf75709a3dd009ead86a33940`): population and daily portal choice are independent; the objective and achievement are unknown. Four creature pads, a connected 49-cell cleared room floor and an adjacent return selector are conservative geometry. Separate maintained dry pads beside each surface entry prevent landing on the active entry. These escape surfaces persist without enabling a closed entry. No objective or achievement is invented.

Hive Outpost uses six Vandura source pads conservatively assigned to the six hive-born types. Daily 500-permille admission and 49:1 ordinary/Overseer weights are explicit assumptions. The optional Overseer occupies a seventh adjacent clear pad and preserves Kollos. The retained [Hive Outpost page](https://tibia.fandom.com/index.php?curid=76241&oldid=887146), line 6 (SHA256 `8abed8a9797761bfd4b722eb5b7b3cd547eb00ee93d028a54cc845f3ac0fc4ca`), establishes Vandura and the invasion, while leaving the reward quantity unknown. Exact donor homes are `world-monster.xml:51169,52098-52102` at Crystal commit `201189f871d23597b82caf88fd611454a2ddefb8`, SHA256 `f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e`.

Poacher Caves uses the same 12 Gloom/Ghost Wolf donor homes and 60-second source intervals for mutually exclusive Game/Poacher/Gloomy populations. The retained [Poacher Caves page](https://tibia.fandom.com/index.php?curid=76258&oldid=1175805), lines 17-18 (line SHA256 `5f657dd641977d669f56568d3f13ce11480418696a162e6817a8b9ff6ebaf0ea` and `86aede34bd487d35f366c6acd47a369d707214110e68b4b2ac083a153f359b3b`), describes the populations but supplies no executable threshold/order. Equal daily selection is the conservative rule. Exact homes are `world-monster.xml:63222,63329,63776,63891,63921,64194,64310,64313,90293,90811,90976,91588` at the same pinned Crystal commit and XML SHA256 above. No kill threshold is invented.

All decisions use the existing World/reset owner. All 63/4/6+1/12 native creature memberships use already-qualified native policies; no additional Creature or Item profile is introduced. Recipe source fields retain the full donor URLs, exact XML home lines and map SHA256 `c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`.

## WVAR Thornfire reset and water stations

Thornfire uses the existing World/reset owner and exact slot15 recipes. The restored phase retains54 pinned Shadowthorn Elf homes on688 admitted tiles, including three Overseers; the burning phase retains24 Firestarters on634 tiles. The explicit conservative phase rule requires one sealed native death from every exact restored home and all three Overseer-release bits. Existing next-save admission maps counter stage1 to restored variant1 and stage2 to burning variant0; one burning save epoch then restores the normal roster. The phase has no random selection or boot activation. Individual repeated kills cannot substitute for another home.

Prepared bucket source facts remain bound to Crystal201189f `prepared_bucket.lua:3-83` (SHA256 `5e0d0253f913cb788a7fcbd6be41ad465b56ab1026c68226f80f79c31cefe01f`) and `thornfire_wolf.lua:3-18` (SHA256 `010270cc7a3ad1ecdd960b645d1013bc61f24feace4df38092295b91aca2b7cf`). Mixing11466 with2873 yields empty bucket12819; using bog water12721 fills it to12722. Each fire use transforms the filled state back to empty; filled water decays after one hour. Existing terrain restoration uses the first future save+8h or save+16h boundary, then the next reset after16h. Every extinguish increments the personal count; Firefighter grants only on the10% two-wolf branch when the count is at least500 (`prepared_bucket.lua:44-59`, same source SHA256 above); no guaranteed mount or unrelated event reward is invented. The retained Lua samples four nearby pads with replacement and permits `createMonster` failure. The explicit conservative placement rule requires two distinct currently available source pads atomically, otherwise the whole use refuses before debit; it does not claim exact donor sampling. Current Character/session/generation/lease, exact selected action membership, atomic Item custody, durable replay and native spawn/death receipts remain with their existing owners.


The final Thornfire admission extends the earlier42-key closure through the same producer to the existing M10 bog-derived Crystal Wolf:43 actor keys,42 retained Creature Lua definitions,137 corpse-chain Items and344 dependencies. Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `data-crystal/monster/mammals/crystal_wolf.lua:32,78-85`, SHA256 `55815115e6b393f6e7cab39a69ee0c59fc0c87acf59dcf9ce1e0e33e4d6e7061`, supplies its six exact loot leaves and12058→12069→12070→12071 corpse chain (capacities10/10/10/leaf); the unrelated modern10310 loot edge is removed. The two bucket profiles remain physical/use operands rather than unrelated loot. Existing craft authoring admits nine operand profiles and130 recipes. Native Item payload is3728 rows/8,107,377 bytes; the official Prey producer was rerun against the final loot bytes (15 cases/776 creatures). Historical42-key PG receipts remain evidence for their recorded payload, and do not substitute for the separate final43-key Native admission proof.

### WVAR expanded compile gate

All15 changed crates passed all-target/all-feature `cargo check` (116.181s).
Two main test fixtures now provide WVAR's new optional bundle/variant fields;
their existing layer and fungus assertions are retained. Prey completed15 cases
for776 creatures against the merged source loot. Clippy, pins and PostgreSQL
remain pending at this compiling checkpoint.

### WVAR regenerated compiling checkpoint

All15 changed crates pass Clippy all targets/features with `-D warnings`
(285.315s). Native registration freshness passes84.339s. Quest-authoring ran
all93 modules in509.049s:92 passed; the sole stale Grizzly hash was then
regenerated and its test passed3.909s. Native/Prey, dependent shop/potion and
static imbuement evidence were regenerated with repository tooling. The new
bounded evidence refresh proves all34,111 canonical Items equal pinned main
plus verified WVAR admissions before invoking existing evidence producers;
257 imbuement tests pass84.997s in the prepared Python environment. Pin refresh
passes249.826s. Pin check and native PostgreSQL execution are still pending;
this checkpoint does not claim WVAR qualified in the package yet.


### WVAR reverted within its packaging window

Reverted merge7d24f6cb4 and its follow-ups832e65001,91f7fbf1f,892cef29d,
982d52b00. The runtime, authored/generated content, pins and migrations now
exactly match the previously green WORLDCHG registry checkpoint796d7b973;
only unioned historical notes/reports differ. WVAR's1334/1335 migrations are
absent. Source contract/report entries are retained as historical evidence.

WVAR could not complete focused qualification in60 minutes. Final red point:
the pin-check process was killed after139.659s and the dispatch PostgreSQL
binary build after279.814s when overlapping memory exceeded the16GiB limit
(cgroup oom_kill increased by2). No WVAR PostgreSQL case executed; no passing
pin check or WVAR qualification is claimed. Future heavy builds and pin
materialization must run serially. Its compile/Clippy, source producer,
Prey simulation, quest-authoring recovery and pin-refresh evidence remain
recorded above for recovery from the pushed history. Continue with WCONTRIB.

Rollback validation: wvar-revert-check PASS 117.904s; wvar-revert-fmt PASS 15.026s.


## Integrated WCONTRIB source contract notes

# WCONTRIB decisions

Horse Station follows the owner's three-hour-or-zero-horses activation rule and three-hour raid expiry. A late observation preserves the original deadline; observing after expiry restores the stable immediately.

This rule intentionally differs from the retained rental-triggered 24-hour raid: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/npc/palomino.lua:104-112,148-160`, SHA256 `595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da`. Native initial stables contain nine horses: `data-global/scripts/globalevents/spawn/horse_station_world_change.lua:1-9,18-22,39-43`, SHA256 `631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b`.

Dog Collar issuance uses an atomic gift: eligibility, earning and native backpack MINT commit together. A full backpack refuses without earning or setting the once flag. A successful collar remains in custody and exact replays remain valid after the disappearance ends; there is no unearned pending grant.

World live changes insert revision one once and advance existing rows with guarded UPDATEs. PostgreSQL executes BEFORE INSERT guards before an UPSERT conflict arm, so UPSERT is deliberately unsuitable for the immutable revision chain.

Steamship requires 200 shared Coal and opens only at the next save; each retiring epoch funds its following saved day. Junkar's retained destinations and prices remain the travel authority.

Swamp Fever uses an owner-authorized conservative threshold of 100 medicine pouches12517 to cure/prevent the following saved day's epidemic. The retained donor establishes the medicine and individual Ottokar exchange, but not a shared numeric cure threshold. The individual exchange remains separate.

Twisted Waters uses 1,000 corpse deposits per retiring epoch, advancing one pollution stage when met and recovering one stage otherwise. Yawno and explicit dirty-water use accept the final portable aged Swamp Troll corpse4178. The selected native lake water component is projected as clean/dirty/stinking; only a successful Shimmer catch starts the sourced personal 20-hour restriction. These deposit/recovery and lake appearance choices are conservative authored rules.

Rathleton uses twelve fixed ballot identities and one earned right per explicit cast. Unknown alternate-Monday parity is anchored to Monday2000-01-03UTC; the active epoch's save timestamp owns the period. Ties retain incumbents. Native missing entrance selectors are replaced by explicit Marvin passages to sourced dungeon/first-pipe destinations, checked against current promoted rank and elected dungeon inside the existing travel transaction. Rank admission also uses native Alaistar/Flint trades and citizen boat routes. Sources and exact hashes are recorded in the owned WORLDCHG-Q rows.

Horse returns restore one shared horse, bounded at nine; restoring all nine ends a raid early. Reduced stable cohorts use the first N members of the sourced nine-horse layout. Existing native raid scheduling, Channel target admission, actor cleanup and generation guards own every cohort.

Noodles uses the accepted92-permille saved-day disappearance rule and thirteen documented Thais roaming anchors. An escort belongs to one Character across Channels, preserves its collar, and can return only beside the actual King. The personal repeat interval is fourteen days. A return guarantees Dog Lady on the next saved day; the next reset releases any unfinished escort and selects the new day's disappearance. Dog Lady uses her existing dog presentation32, never an invented human appearance.

The shared NPC reward/access engine is reused from the quest-engine lane at5a6de28c142de185594217ce1e5cbc7f4b6e6e4e. Its unrelated binary map restoration is excluded. Required WCONTRIB NPCs are bound through the existing retained-source placement converter; missing legacy travel-service declarations use the native route catalogue and greeting adapter.

King `noodles` dialogue and collar use share the same World RETURN action. Spatial commits and receipt observation bind the original player/NPC context; ambiguous outcomes retain that context instead of reconstructing it after either actor moves.

Ottokar's personal exchange remains distinct from epidemic donations: one medicine pouch (12517) yields one belongings item (12413). Repeated custody-backed gifts use exact per-command claim identities; an exact replay never consumes or awards again. The hundredth personal delivery grants Doctor! Doctor! once; the exchange count can keep growing while the donor achievement progress itself saturates. Retained Crystal9f5a72c ottokar.lua:61-77 SHA256fe8c4a6740a75448c3af035f27c3b2783b9fe91ecad7ba4d255544003370a951; data/scripts/lib/register_achievements.lua:696-717 SHA25624ca189087f6403ab994720763b69ee04eab0397ff917ff7700edb9063d6de37. The belongings profile retains items.xml:30161-30163 weight3000, with the existing source XML hash cited in WORLDCHG-Q.

Alaistar's Commoner shop uses the retained glob-of-glooth21182 sale for125 gold: Crystal9f5a72c data-global/npc/alaistar.lua:48 SHA256632237084e0382dd959172a0f7d737ed747fad710a2cea9db51b7f849bd8c36c. The forward content repair restores the offer removed by the earlier unqualified-access filter; the current promoted-rank predicate now gates its real transaction.


### WCONTRIB compiling merge checkpoint

Sourcec8a8d97cf is integrated with WORLD behavior retained. Core server, dispatch
and World-change libraries check all features in30.531s. World-change
unit/component tests pass; twelve quest/cosmetic producer tests pass5.634s.
Main's14 outfit definitions, palette, fallbacks and8 mounts remain authoritative;
49 new physical looks produce63 unique outfit pairs. Legacy grants and new
canonical quest grants resolve to the same retained identities. Existing WORLD
callbacks receive empty new effect/projection fields. Skinning, current native
teleports, dynamic quest floor masks, main static layers and NPC dialogue digests
are combined with the new shared state, fishing and Quest participants.

Own migrations1336–1339 consolidate source1110–1115 in original SQL order.
0999 is the exact canonical QENGINE2 dependency at aad7ff6dd, with no own variant;
1000–1006 are not needed by this lane and are not imported. If3020 merges into
main later, consume main and compose the newer owner proofs then. Source notes
are unioned only here; no root CONTRACT-NOTES.md remains.

Full legacy/native regeneration, final pins and PostgreSQL qualification remain
pending. No package QUALIFIED claim is made at this compiling WIP checkpoint.


### WCONTRIB legacy and compatibility checks

Full legacy materialization and Rust inventory repinning pass567.363s; NPC
placement regeneration/check and converter tests pass0.147/0.151/0.266s.
All12 changed crates pass all-target/all-feature check51.015s. The complete
World owner test module now passes10 cases, including positive/negative main
static Fury requirements. Furyosa keeps main's independently admitted, persisted
static layer guard; full WVAR remains reverted. Node epoch, exit-only, undated
and active-layer checks are retained, and WORLD's negative fixture chooses
truly inactive Fury days.25 cosmetic compiler tests pass2.99s. Main's49
original item/base mappings and14 outfit records remain intact; additional
Quest mappings have the closed QENGINE provenance.

Native registration now orders WCONTRIB after the accepted ITEMHOLD and QENGINE
owners. Only exact matching custody/binding/admission facts may reuse14156 or
21182; their existing presentation, semantics and provenance are preserved.
New native/model outputs, final pins, Clippy and real PostgreSQL remain pending.

### WCONTRIB producer encoding and lint checkpoint

Clippy all12 changed crates/all targets/all features passes253.594s with
`-D warnings`. The Quest reward producer now uses the shared native manifest
encoder, keeping its output byte-identical to ordinary native regeneration.
Its12 tests pass5.827s and all25 cosmetic compiler tests pass2.879s. This also
retains main's bounded external cosmetics descriptor instead of embedding the
large source shards. Final native model, pins and PG remain pending.
