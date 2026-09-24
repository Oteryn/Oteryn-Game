# Native source first-slice resource envelope

- Decision: `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1`
- Status: **CANDIDATE; acceptance requires independent review and protected integration**
- Escalation: #336; source readiness #319; coordinator #162
- Immutable base: `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd`
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
source_escalation: 336
blocking_question: Which finite first-slice bounds permit native evidence and assignment adapter qualification?
facts:
  proven: [accepted_330_requires_applicable_finite_bounds, registry_has_no_complete_native_source_mapping, existing_fixed_identity_and_crypto_bounds_cover_some_fields]
  derived: [HTTP_and_assignment_resources_need_their_own_accounting_envelope]
  unknown: [measured_latency_and_capacity, actual_counterpart_certificate_sizes, production_source_availability]
  conflict: []
accepted_decision: NATIVE-SOURCE-RESOURCE-ENVELOPE-V1, conditional on protected integration
rejected_options: [reuse_unrelated_game_frame_limits, unbounded_library_defaults, automatic_retry_fanout, unlimited_cache_or_queue, delete_durable_floors_to_bound_memory]
affected_contracts: [FND-NATIVE-SOURCE-EVIDENCE-V1, OPS-SCOPE-ASSIGNMENT-FENCING-V1]
affected_paths: [this_decision, OTV2-20260906-native-source-bounds-recovery-336_task]
implementation_owner: Work separately allocates registry registration and Game adapter implementation
implementation_scope: bounded first-slice source and assignment consumers only
resource_values_changed: true
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: [only_unresolved_resource_values_for_these_first_slice_330_operations]
required_validation: max_max_plus_one_overflow_slow_peer_and_combined_accounting
required_independent_review: exact-head resource_and_security_boundary_review
next_action: Work independently qualifies the candidate before separate registry and implementation allocations.
```

## Why decide now

**Must decide now: YES.** Accepted #330 requires finite native private transport/assignment bounds before executable acceptance. Its five-second source-age policy does not terminate a hung request, bound headers or constrain a queue. Pure semantics and B #329/#335 remain independent.

These are conservative **first-slice policy proposals, not measured production capacities**. Select a single configured producer with two simultaneous evidence exchanges: enough to obtain security and trust concurrently without creating a general fan-out client. Small queues absorb a bounded arrival burst; excess work explicitly fails closed. Correctness/abuse safety motivates the bounds, not a player-count or throughput claim. Real latency, certificate-chain and workload evidence may justify a later reviewed amendment; incompatible sources are unavailable rather than an excuse to bypass a cap.

Deliberately excluded: compression, batching, enumeration, automatic retries, HTTP/2 multiplexing, redirects, automatic endpoint discovery, platform write/deployment authority, certificate provisioning, service-level targets and global database retention policy. Enabling one later requires its own explicit bounded contract. No generic transport/framework is selected.

## Existing exact mappings

Reuse existing bounds only for the identical semantic value: native UUID identity/encoding; FND-04 key ID grammar (1–64 ASCII); exact configured issuer/profile/purpose constants; Ed25519 public key (32 decoded bytes/43 unpadded base64url characters); #330 positive uint64 revision/generation and nonnegative uncertainty encodings (at most 20 decimal digits), nonnegative signed-64 Unix time (at most 19 digits). These are representation bounds, not HTTP body budgets. Validate before retaining decoded values, including excessive leading-zero/escape encodings.

No FND02 frame/protobuf, catalogue, telemetry or admitted-session queue value is imported. Native adapter accounting is distinct; shared infrastructure must charge each applicable budget rather than granting extra unaccounted capacity. The accepted ≤5-second source-age rule remains independent of every execution timeout below.

## Proposed registry rows: evidence transport and decoding

All byte units below are octets; KiB means 1,024 bytes. Limits are inclusive, configurable downward where meaningful, never above the listed maximum. The IDs are proposed registration names, not registry edits in this candidate. Each cap bounds adapter-owned input/state/work; it is not a claimed total process RSS limit. Exact allocator/TLS-library overhead must be bounded and measured in implementation qualification, without introducing any unbounded hidden collection.

| Proposed ID | Hard maximum and accounting scope | Rationale / enforcement |
|---|---|---|
| `NSRC-HTTP-HEADERS` | 8,192 bytes per request/response start-line + header section; 32 fields; 2,048 bytes per field line; response status line 256 bytes | Fixed small private operation needs few headers; reject while reading, before buffering excess. Header names/values count raw bytes including delimiters. |
| `NSRC-HTTP-BODY` | 8,192 dechunked response bytes; request body 1,024 bytes | Closed scalar envelope is substantially smaller; this provides encoding headroom without game-frame-sized allocations. Content-Length above cap rejects immediately. |
| `NSRC-HTTP-CHUNKS` | 64 data chunks; 4,096 total transfer-framing bytes; 64 bytes per chunk-size line | Supports bounded unknown-length/chunked streaming; every byte and chunk charged before retention. Framing excludes data and headers. |
| `NSRC-JSON-SHAPE` | Root object only; at most 16 members; no nested containers/arrays; member name 64 decoded ASCII bytes; string 256 decoded bytes unless a smaller field rule applies | Largest #330 observed family has 14 fields; 16 is bounded parser capacity, not permission for unknown fields. Streaming validation rejects a nested container/unknown key before materializing it. |
| `NSRC-SOURCE-AUTHORITY` | 128 ASCII bytes matching `[A-Za-z0-9._:/-]+` | Named service identity, not arbitrary prose or discovery URI. Exact authenticated descriptor equality still required. |
| `NSRC-DESCRIPTOR` | One active producer descriptor per process; 4 operation mappings; origin 256 ASCII bytes, each path 256 ASCII bytes, each service/client identity 128 ASCII bytes; total noncertificate descriptor 4,096 bytes | Four fresh/recovery operations fit one producer; no endpoint/key fan-out. Authenticated configuration validation rejects over-limit inputs. |
| `NSRC-TLS-CERTS` | Peer chain 4 certificates; 4,096 DER bytes each; 16,384 aggregate DER bytes; configured trust roots 4 certificates with the same per-cert/aggregate cap | Explicit conservative PKI interoperability envelope; chain outside it is unavailable until reviewed, never silently truncated. |
| `NSRC-TLS-VERIFY` | DER nesting 16; 512 parsed DER elements per certificate; 4 certificate-signature verifications per handshake; 65,536 inbound handshake bytes across all reads | Bound hostile parser/path work as well as retained chain bytes. Pre-index the configured trust set; no network AIA/CRL fetch or unbounded alternate-chain search. Verified current service-PKI state remains required. |
| `NSRC-INFLIGHT` | 2 total end-to-end evidence pipeline slots per logical adapter registration, including HTTP, SQL publication, completion and ambiguity/reconciliation; at most one HTTP request per slot; no idle connection pool | Security/trust pair may proceed concurrently. Decode does not release a slot. A slot ends only after definitive result classification, normalized completion consumption and release of all owned resources. |
| `NSRC-QUEUE` | 8 pending requests; 1,024 retained bytes each; 8,192 aggregate queued bytes per process | Fixed small burst budget; no hidden future/waiter list outside accounting. Active work is charged separately. |
| `NSRC-ACTIVE-BUFFERS` | 131,072 retained adapter-managed bytes per end-to-end pipeline slot | Covers simultaneous bounded header/body/handshake/certificate/SQL submission/result/completion/reconcile buffers and copies; two pipeline slots cap these buffers at 262,144 bytes. Library bookkeeping must have a separately demonstrated finite bound, not be hidden in an unbounded buffer. |
| `NSRC-PENDING-PUBLICATION` | 2 retained pending/ambiguous publication identities; 16,384 bytes per durable checkpoint, 32,768 aggregate per logical adapter registration; one completion mailbox per slot, 2,048 bytes each | Checkpoint exact immutable operation binding before SQL submission. Each slot has at most one outstanding SQL operation and one normalized result; duplicates do not enqueue additional objects. |
| `NSRC-PROJECTION` | 64 resident typed observation entries; 2,048 retained bytes each; 131,072 aggregate bytes per process | Small first-slice working set. Missing/evicted observation closes that lookup until asynchronously reloaded and revalidated. Durable floors/denials/receipts are never evicted or deleted by this cache limit. |

Use HTTPS/TLS 1.3 with HTTP/1.1 only for this resource profile; do not negotiate HTTP/2. Every connection carries one exchange and closes afterwards. Reject conflicting/duplicate framing headers, Content-Length plus Transfer-Encoding, unsupported transfer codings, nonempty trailers, redirects and informational responses. `Content-Encoding` may be absent or identity only. No decompressor is created. Unknown Content-Length is accepted only as bounded chunked or EOF-delimited HTTP/1.1 body; read at most one excess byte to detect overflow without retaining it. Chunk-size parsing uses checked arithmetic and rejects oversized announcements before body allocation. EOF/truncation is not successful JSON completion. Empty body remains subject to the required response family.

Raw JSON whitespace and escapes count against total body bytes; decoded field values additionally obey semantic lengths. No generic DOM may first allocate a large unknown field before rejection. Larger valid-but-noncanonical encoding is not an interoperability entitlement. HTTP request targets and configured identities are validated/charged before connection setup.

TLS byte/cert/parser/work limits must be demonstrably enforceable by the selected library/adapter before acceptance. A lower layer that allocates or performs unbounded verification before the cap fails qualification. No claim is made that the current library already exposes all necessary controls; verified inability requires a bounded implementation finding, not silent omission. Local trust-root count bounds do not replace authenticated PKI/bootstrap/revocation semantics from #330.

## Execution, cancellation and retry budget

| Proposed ID | Hard maximum | Meaning |
|---|---|---|
| `NSRC-QUEUE-WAIT` | 1,000 milliseconds | Queue residence on a trusted monotonic clock; expiration releases pending entry and reports unavailable. |
| `NSRC-CONNECT` | 1,000 milliseconds | Connection establishment after dispatch; contained in the total exchange deadline. |
| `NSRC-HANDSHAKE` | 2,000 milliseconds | TLS handshake after transport establishment; also contained in the total exchange deadline. |
| `NSRC-EXCHANGE` | 3,000 milliseconds | Dispatch through complete response authentication/decoding; includes connection, handshake, headers/body and parser work, not their sum. |
| `NSRC-PUBLISH` | 2,000 milliseconds | One asynchronous guard publication/reconciliation operation, including database acquisition; never extends evidence age. |

These are deliberately finite availability controls with no measured latency claim. A slower healthy dependency may be rejected; that is the explicit first-slice cost. Five-second evidence age is still checked from the actual authenticated source observation and can reject before any operation timeout. Publication can be ambiguous on timeout and must reconcile its original binding; cancellation is not proof of rollback.

**End-to-end ownership is mandatory.** Acquiring a pipeline slot reserves all later publication/completion/reconciliation capacity. Fast HTTP cannot free it while SQL or a consumer is stalled. The exact accepted source revision/decision, purpose, publication CAS and immutable operation binding remain owned by the same slot through an ambiguous outcome; a timeout cannot create a detached publication or another identity. Reconciliation uses that occupied slot, after the prior SQL operation is proven finished/canceled or through its authoritative result, and never overlaps an unaccounted outstanding operation. The two-slot limit bounds SQL submissions and completion objects as well as HTTP.

Before SQL submission, persist the bounded pending-slot checkpoint under a stable logical adapter registration with two fixed slot identities. That reservation/checkpoint write itself occupies the slot; an ambiguous checkpoint result is reconciled by its fixed identity before further work. Only one such active registration is allowed for this process/producer configuration; inventing a new registration to bypass occupied slots is forbidden. Restart reconciles occupied checkpoints before admitting new pipeline work; process reincarnation cannot allocate two fresh slots while old ambiguous publications remain outstanding. Competing incarnations require fenced exclusive registration custody and database-visible slot reservation. Missing/unprovable checkpoint state keeps the registration closed. This is a bounded execution/recovery index, not source authority or permission to delete durable receipts/floors. Definite historical completion may release the execution slot after normalized handling while the canonical receipt remains durable. If both slots remain ambiguous, new evidence demand is unavailable; safety bounds take precedence over availability.

Automatic retries, background refresh loops and coalesced waiter fan-out are disabled in this slice. Each demand is a separately counted submission; a caller may submit again only through the same bounded admission queue. No retained retry list exists. A slot is not reusable while canceled work continues consuming its buffers/socket/verification task; if cancellation cannot promptly reclaim it, the slot remains unavailable and cannot spawn replacement work. Bounded CPU/parser work must cooperate with deadlines; abandoning a waiter does not detach uncounted work.

## Proposed registry rows: assignment command boundary

No new HTTP assignment service or command batch/enumeration is introduced. An authenticated, authorized control actor invokes the accepted typed single-scope operation; the ordinary runtime identity cannot assign itself.

| Proposed ID | Hard maximum and representation | Rationale / behavior |
|---|---|---|
| `NASG-OPERATION-KEY` | Exactly 32 opaque bytes, encoded as exactly 43 canonical unpadded base64url characters at a text boundary | Stable 256-bit idempotency identity; not a bearer credential or new account/entity ID. Issued once by the authorized control operation and bound to the exact command. |
| `NASG-COMMAND-BYTES` | 1,024 retained/encoded bytes per command, including actor/scope/operation/predecessor/target bindings; actor/source identities ≤128 ASCII bytes each | Fixed typed fields fit without generic metadata. Oversize rejects before enqueue or transaction. |
| `NASG-QUEUE` | 8 pending commands and 8,192 aggregate retained bytes per assignment-writer process | One bounded control backlog; no hidden waiter list. |
| `NASG-INFLIGHT` | 1 end-to-end assignment operation slot per logical writer registration, held through result/completion and ambiguity/reconciliation; 4,096 retained typed command/result/checkpoint bytes | Intentional first-slice serial control execution; independent replicas still use canonical database CAS, not this local limit, for authority exclusion. |
| `NASG-EXECUTION` | Queue wait 1,000 ms; dispatched operation 3,000 ms including DB/lock acquisition and response | Finite control work, no lease/grace value. Timeout is ambiguous until original operation reconciliation. |

Read/reconcile handles one exact scope or operation, returning one result bounded by 4,096 bytes. Enumeration, multi-scope commands, automatic retries and retained retry lists are unsupported. Reconcile uses the same queue/in-flight accounting; write timeout does not authorize a new operation key or reset source/generation marks. The single assignment slot follows the same no-detachment/restart rule: persist its exact operation binding before submission, retain it on ambiguity, fence registration custody, and reconcile it before admitting new work. Completion has one bounded mailbox within the same 4,096-byte envelope; no extra result/waiter queue is allowed. Persistent receipts/floors remain required; these memory/operation bounds are not a lifetime database-row cap or retention permission.

## Registration, failure and qualification

After acceptance, Work must separately allocate exact registry rows/mappings and adapter paths. The registration must carry each dimension, accounting unit/scope, downward configuration range, failure category and boundary tests; compound table rows may be split into individual machine-readable entries. Registry integration precedes implementation acceptance. No registry file is changed here.

Structural/coding violations reject as invalid input; exhausted queue/byte/work capacity rejects or backpressures before resource reservation; transport/time dependency failure is unavailable. Authentication failure never becomes retry success; source staleness/revocation retains the existing FND-04 categories. Assignment timeout retains ambiguous/reconcile semantics. Aggregate checks use checked arithmetic, include copied/queued objects and release accounting exactly once. No partial authority mutation is allowed to meet a budget.

Required proof: each inclusive maximum and maximum-plus-one; arithmetic overflow; body/header/chunk attacks; duplicate/unknown/nested JSON and escape expansion; excess cert/path work; slow partial TLS/HTTP; queue expiry; cancellation without slot reuse; combined simultaneous caps; cache eviction plus restart with durable floors; fast HTTP with stalled SQL/completion remaining bounded at two retained pipelines; crash with both pending slots occupied; repeated timeout/restart without capacity multiplication; assignment lost-response reconciliation retaining its sole slot; and no uncounted retry/coalescing work. Use actual authenticated transport and configured PostgreSQL integration harness, with independent source mutation tests. Tests establish only their environment, not production source availability.

This narrowly fills #330's resource deferral and selects HTTP/1.1 for this profile. It changes no existing registry row or unrelated consumer limit, grants no Platform acceptance/write, and leaves actual source/bootstrap/character-operation authorization dependencies explicit. Document validation is governance/whitespace/self-review and independent exact-head review plus canonical CI; runtime/E2E is NOT_APPLICABLE to these docs.

## 8. First-slice configuration addendum — 2026-09-06 / #342

This addendum records the separately dispatched Supervising Architect resolution of #342, escalation comment `5558989135` and resolution/allocation comment `5559004684`. It becomes accepted only through this package's independent review and protected integration. It supersedes only the first-slice downward-configurability wording in section 3 and the downward-range registration requirement in section 7; the historical text above remains preserved.

For the initial implementation of `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1`, the only supported resource configuration is the complete profile already specified by this decision. For each independently registered numeric resource-cap dimension with accepted hard maximum M, register `hard_maximum: M` and `configurable_range: {minimum: M, maximum: M}`. This range constrains the configured cap; it does not require actual usage, message length, queue occupancy, certificate count or elapsed execution time to equal M. Existing exact-size, grammar, identity and semantic validity requirements remain independently binding.

Smaller configured profiles are unsupported in this first slice. This explicitly defers the downward configurability previously described as “where meaningful.” No zero/one minimum or smaller profile is authorized. A later bounded amendment may accept a smaller profile only with named need and complete evidence for its coupled limits, meaningful valid-operation interoperability, accounting, timeout containment, failure behavior and restart/ambiguity custody.

Every existing hard maximum, accounting scope, transport restriction, source-age/provenance rule, slot reservation and custody obligation, retained durable floor/receipt obligation and failure classification remains binding. Registration neither activates runtime defaults nor proves enforcement, production capacity or actual source availability.

**Must decide now: YES.** Registry #342 cannot populate required configuration ranges faithfully while lower endpoints remain unspecified. Smaller ranges need not be selected now: no demonstrated source interoperability or product requirement needs them. The cost is no downward cap tuning in the first slice; slow dependencies may remain unavailable under already accepted deadlines. Reopen on a concrete constrained deployment, valid producer incompatibility, security finding or measured resource behavior. No general configuration policy, technology, capacity target, retention policy or external write authority is selected. Validate exact dimensional mapping and independent review before protected acceptance; runtime qualification remains the owning implementation's separate obligation.
